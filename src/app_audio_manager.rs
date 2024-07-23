use std::{cell::RefCell, collections::VecDeque, iter::zip, rc::Rc, sync::{mpsc::{self, Receiver, Sender, TryRecvError}, Arc, Condvar, Mutex}, thread::{self, JoinHandle}};
use rustfft::{num_complex::Complex, FftPlanner};
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};
use windows::{core::{implement, Interface}, Win32::{Media::Audio::{eMultimedia, eRender, IAudioSessionControl, IAudioSessionControl2, IAudioSessionManager2, IAudioSessionNotification, IAudioSessionNotification_Impl, IMMDeviceEnumerator, MMDeviceEnumerator}, System::Com::{CoCreateInstance, CLSCTX_ALL}}};
use wasapi::*;

use crate::FFT_FREQUENCY;
use crate::common_audio_manager::FftHandler;

// TODO: Maybe set thread priority to high

struct AudioThread {
    device_id: String,
    audio_client: AudioClient,
    format: WaveFormat,
    playing: Arc<(Mutex<bool>, Condvar)>,
    device_change: Sender<bool>,
    handler: FftHandler,
}

impl AudioThread {
    pub fn new(sample_destination: Sender<Vec<(Complex<f32>, f32)>>, fft_planner: Arc<Mutex<FftPlanner<f32>>>, playing: Arc<(Mutex<bool>, Condvar)>, device_change: Sender<bool>, process_id: Pid) -> Self {
        // Get device and client
        let device = get_default_device(&Direction::Render).unwrap();
        let device_id = device.get_id().unwrap();
        // let mut audio_client = device.get_iaudioclient().unwrap();
        let mut audio_client = AudioClient::new_application_loopback_client(process_id.as_u32(), true).unwrap();

        // Set desired format
        let format = WaveFormat::new(32, 32, &SampleType::Float, 44100, 1, None);

        // Gather information about format and client
        // let (_def_time, min_time) = audio_client.get_periods().unwrap();

        // Initialize client
        audio_client.initialize_client(
            &format,
            0,
            &Direction::Capture,
            &ShareMode::Shared,
            true,
        ).unwrap();

        // Plan FFT for this format
        let fft = fft_planner.lock().unwrap().plan_fft_forward(format.get_samplespersec() as usize / FFT_FREQUENCY as usize);

        // Create the FFT handler
        let handler = FftHandler::new(sample_destination, format.get_samplespersec(), fft);

        AudioThread { device_id, audio_client, format, playing, handler, device_change }
    }

    pub fn capture_loop(&mut self,) {
        // Gather information about client
        let event_handler = self.audio_client.set_get_eventhandle().unwrap();
        let capture_client = self.audio_client.get_audiocaptureclient().unwrap();

        // Gather information about format
        let block_align = self.format.get_blockalign();
        let chunk_size = self.format.get_samplespersec() as usize / FFT_FREQUENCY as usize;
    
        // Create queue for sending samples
        let mut sample_queue: VecDeque<u8> = VecDeque::new();

        // Block until we first want to start stream
        let (lock, cvar) = &*self.playing;
        let mut playing = lock.lock().unwrap();
        while !*playing {
            playing = cvar.wait(playing).unwrap();
        }
        drop(playing);

        self.audio_client.start_stream().unwrap();

        // Allocate memory as f32 to ensure correct alignment
        let mut data = Vec::with_capacity(chunk_size);

        // Main loop
        loop {
            while !sample_queue.is_empty() {
                // Temporary buffer for samples
                let mut temp = Vec::with_capacity(std::cmp::min(sample_queue.len(), (chunk_size - data.len()) * 4));

                // Try to fill the chunk with samples
                while sample_queue.len() > 0 && temp.len() < temp.capacity() {
                    temp.push(sample_queue.pop_front().unwrap());
                }

                // Convert to f32
                for chunk in temp.chunks(4) {
                    let mut sample = [0; 4];
                    sample.copy_from_slice(chunk);
                    let sample = f32::from_ne_bytes(sample);
                    data.push(sample);
                }

                // Ensure the chunk is filled before further processing
                if data.len() != chunk_size {
                    break;
                }

                // Perform FFT on the data
                self.handler.perform_fft(data.as_slice());

                // Remove the first quarter of the samples, this is done to smooth the visualisation by creating overlapping windows
                data.drain(0..data.len() / 4);
            }

            let new_frames = capture_client.get_next_nbr_frames().unwrap_or(Some(0)).unwrap_or(0);
            let additional = (new_frames as usize * block_align as usize).saturating_sub(sample_queue.capacity() - sample_queue.len());
            sample_queue.reserve(additional);
    
            // Read from device to queue
            if new_frames > 0 { capture_client.read_from_device_to_deque(&mut sample_queue).unwrap(); }
            if event_handler.wait_for_event(1000000).is_err() {
                self.audio_client.stop_stream().unwrap();
                break;
            }

            // If this device is no longer active, we should kill this thread and a new one should be started with the new device
            let test = get_default_device(&Direction::Render).unwrap().get_id().unwrap();
            if self.device_id != test {
                let _ = self.device_change.send(true);
                return;
            }
    
            // Check if we should stop the stream and if so, wait for command to start again
            let (lock, cvar) = &*self.playing;
            let mut playing = lock.lock().unwrap();

            if !*playing {
                self.audio_client.stop_stream().unwrap();
                while !*playing {
                    playing = cvar.wait(playing).unwrap();
                }

                self.audio_client.start_stream().unwrap()
            }
        }
    }
}

/// Holds all necessary information for the app audio manager.
pub struct AppAudioManager {
    current_handle: Option<JoinHandle<()>>,
    sample_destination: Sender<Vec<(Complex<f32>, f32)>>,
    playing: Arc<(Mutex<bool>, Condvar)>,
    fft_planner: Arc<Mutex<FftPlanner<f32>>>,
    device_change: Receiver<bool>,
    monitor: AppMonitor,
    current_pid: Option<Pid>,
}

impl AppAudioManager {
    /// Initialises and creates a new app audio manager.
    /// 
    /// # Arguments
    /// 
    /// * `sample_destination` - Is the destination to send the samples for rendering.
    pub fn new(sample_destination: Sender<Vec<(Complex<f32>, f32)>>) -> Self {
        // Create FFT planner
        let fft_planner = Arc::new(Mutex::new(FftPlanner::new()));

        // Condvar for thread control
        let playing = Arc::new((Mutex::new(false), Condvar::new()));

        // Communications channel for reviving thread on device change
        let (_, device_change): (Sender<bool>, Receiver<bool>) = mpsc::channel();

        // Create monitor for opened applications
        let monitor = AppMonitor::new();

        AppAudioManager { current_handle: None, sample_destination, playing, fft_planner, device_change, monitor, current_pid: None }
    }

    /// Starts the audio stream passing samples to the FFT processor.
    pub fn start(&mut self) {
        let (lock, cvar) = &*self.playing;
        let mut playing = lock.lock().unwrap();
        *playing = true;
        cvar.notify_all();
    }

    /// Stops the audio stream passing samples to the FFT processor.
    pub fn stop(&mut self) {
        let (lock, cvar) = &*self.playing;
        let mut playing = lock.lock().unwrap();
        *playing = false;
        cvar.notify_all();
    }

    /// Checks if the audio thread is still alive.
    /// 
    /// If the audio thread has died for some reason, it will be created in the correct state.
    pub fn check_device(&mut self, pid: Pid) {
        match self.device_change.try_recv() {
            Ok(value) => { if value { self.create_thread(pid); } }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => { self.create_thread(pid); }
        }
    }

    /// Returns whether the audio is currently playing.
    pub fn is_playing(&self) -> bool {
        let (lock, _) = &*self.playing;
        let playing = lock.lock().unwrap();
        *playing
    }

    /// Creates the audio thread.
    fn create_thread(&mut self, pid: Pid) {
        let fft_planner = self.fft_planner.clone();
        let sample_destination = self.sample_destination.clone();
        let playing = self.playing.clone();
        self.current_pid = Some(pid);

        // Communications channel for reviving thread on device change
        let (transmit, device_change): (Sender<bool>, Receiver<bool>) = mpsc::channel();
        self.device_change = device_change;

        self.current_handle = Some(thread::Builder::new()
            .name("Capture".to_string())
            .spawn(move || {
                let mut audio_thread = AudioThread::new(sample_destination, fft_planner, playing, transmit, pid);
                audio_thread.capture_loop();
            }
        ).unwrap());
    }

    /// Returns the names of all audio producing applications.
    pub fn opened_applications(&mut self) -> Vec<(String, Pid)> {
        self.monitor.get_opened_info()
    }

    pub fn update(&mut self, app: (String, Pid)) {
        self.check_device(app.1);

        if Some(app.1) == self.current_pid { return }
    }
}

/// Custom callback for when new audio sessions are created.
#[implement(IAudioSessionNotification)]
struct AudioSessionNotification {
    current_apps: Rc<RefCell<Vec<Pid>>>,
}

impl AudioSessionNotification {
    pub fn new(current_apps: Rc<RefCell<Vec<Pid>>>) -> Self {
        AudioSessionNotification { current_apps }
    }
}

impl IAudioSessionNotification_Impl for AudioSessionNotification {
    fn OnSessionCreated(&self, newsession: Option<&IAudioSessionControl>) -> windows_core::Result<()> {
        // Get process ID
        let session = newsession.unwrap().clone();
        let session2: IAudioSessionControl2 = session.cast().unwrap();
        let pid = unsafe { Pid::from_u32(session2.GetProcessId().unwrap()) };
        
        // Ensure the PID is not already in the list
        if !self.current_apps.borrow().contains(&pid) {
            self.current_apps.borrow_mut().push(pid);
        }

        Ok(())
    }
}

struct AppMonitor {
    current_apps: Rc<RefCell<Vec<Pid>>>,
    system: System,
    session_manager: IAudioSessionManager2,
    notification: IAudioSessionNotification,
}

impl AppMonitor {
    pub fn new() -> Self {
        // Get PIDs of all audio producing applications
        let current_apps = Rc::new(RefCell::new(Vec::new()));
        let session_manager: IAudioSessionManager2;
        let notification: IAudioSessionNotification;

        unsafe {
            // Get default audio endpoint
            let device_enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap();
            let device = device_enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia).unwrap();
            session_manager = device.Activate(CLSCTX_ALL, None).unwrap();

            // Create and refresh system for notifications
            let mut system = System::new();
            let refresh = RefreshKind::new().with_processes(ProcessRefreshKind::everything());
            system.refresh_specifics(refresh);

            // Register with audio notifications
            notification = AudioSessionNotification::new(current_apps.clone()).into();
            session_manager.RegisterSessionNotification(&notification).unwrap();

            // Enumerate all applications with an open audio session
            let enumerator = session_manager.GetSessionEnumerator().unwrap();
            let count = enumerator.GetCount().unwrap();
            for i in 0..count {
                let session = enumerator.GetSession(i).unwrap();

                // Get process ID
                let session2: IAudioSessionControl2 = session.cast().unwrap();
                let pid = Pid::from_u32(session2.GetProcessId().unwrap());

                // Skip idle application
                if pid == Pid::from_u32(0) { continue; }

                current_apps.borrow_mut().push(pid);
            }
        }

        // Create and refresh system
        let mut system = System::new();
        let refresh = RefreshKind::new().with_processes(ProcessRefreshKind::everything());
        system.refresh_specifics(refresh);

        AppMonitor { current_apps, system, session_manager, notification }
    }

    pub fn get_opened_info(&mut self) -> Vec<(String, Pid)> {
        // Refresh system
        let refresh = RefreshKind::new().with_processes(ProcessRefreshKind::everything());
        self.system.refresh_specifics(refresh);

        zip(self.get_opened_names(), self.get_opened_pids()).collect()
    }

    /// Returns the names of all audio producing applications.
    fn get_opened_names(&mut self) -> Vec<String> {
        let mut names = Vec::new();
        for app in self.current_apps.borrow().iter() {
            let process = self.system.process(*app);
            if process.is_none() { self.current_apps.borrow_mut().retain(|x| x != app); }
            else { names.push(process.unwrap().name().to_string()); }
        }

        names
    }

    /// Returns the PIDs of all audio producing applications
    fn get_opened_pids(&self) -> Vec<Pid> {
        let mut pids = Vec::new();
        for app in self.current_apps.borrow().iter() {
            let process = self.system.process(*app);
            if process.is_none() { self.current_apps.borrow_mut().retain(|x| x != app); }
            else { pids.push(*app); }
        }
        
        pids
    }
}

impl Drop for AppMonitor {
    fn drop(&mut self) {
        // Unregister from audio notifications
        unsafe { self.session_manager.UnregisterSessionNotification(&self.notification).unwrap(); }
    }
}