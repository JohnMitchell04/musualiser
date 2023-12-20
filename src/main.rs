use std::{
    fs::File,
    io::BufReader
};
use rodio::{Decoder, OutputStream, source::Source, Sink};

mod initialisation;

fn main() {
    // Create our application
    let app = initialisation::initialise_appplication();

    // Try to get the default sound device
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    
    // Load file
    let file = BufReader::new(File::open("./test/titanium-170190.mp3").unwrap());
    let source = Decoder::new(file).unwrap();
    let test = source.pausable(false);
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.append(test);

    app.main_loop(move |_, ui| {
        ui.show_demo_window(&mut true);
    });
}