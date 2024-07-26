# Musualiser

Musualiser is an educational imgui application designed to both play audio files and capture application audio to render their waveforms in real time. This project was pivotal in expanding my understanding of cross-threaded Rust applications, particularly in the use of mutexes, channels, condvar, and arc. Musualiser creates the visualisation by performing an FFT (Fast Fourier Transform) on the audio using the [RustFFT](https://github.com/ejmahler/RustFFT) crate, it then interpolates these results using splines from the [splines](https://github.com/hadronized/splines) crate, and renders them using Bezier curves with [imgui-rs](https://github.com/imgui-rs/imgui-rs). In the process of creating this project I realised no libraries existed for capturing audio from only one application on Windows and that to achieve this people had to interact directly with the Windows API using the [windows-rs](https://github.com/microsoft/windows-rs) crate. Thus I contributed to the [wasapi-rs](https://github.com/HEnquist/wasapi-rs) crate as is detailed [here](https://github.com/HEnquist/wasapi-rs/pull/28).

## Learning Objectives
- **Cross-threaded Rust Applications**: Develop a strong understanding of using mutexes, channels, condvar, and arc for safe concurrency in Rust.
- **FFT and Visualization**: Learn how to perform FFT on audio data and visualize the frequency spectrum using splines and Bezier curves.
- **Audio Processing**: Gain insights into audio rendering and capturing techniques.
- **WASAPI Implementation**: Understand and contribute to the WASAPI project, specifically implementing application-specific audio loopback.
- **Windows Interaction**: Use of the windows API with the windows-rs crate to gather information about and update audio producing apps.

## Dependencies
Musualiser leverages the following libraries:
- [imgui-rs](https://github.com/imgui-rs/imgui-rs) for the GUI.
- [glow](https://github.com/grovesNL/glow) for rendering.
- [winit](https://github.com/rust-windowing/winit) for window management.
- [rodio](https://github.com/RustAudio/rodio) for audio playback.
- [WASAPI](https://github.com/ryanisaacg/wasapi-rs) for recording audio from applications.
- [windows-rs](https://github.com/microsoft/windows-rs)
- [RustFFT](https://github.com/ejmahler/RustFFT) for performing fast fourier transforms.
- [splines](https://github.com/hadronized/splines) for spline interpolation.

## Usage
Run the application using cargo. Note that Musualiser can only run on Windows.

```sh
cargo run --release
```

Ensure your system has OpenGL installed to avoid any rendering issues.

## Example
Capturing the audio from Spotify.
![image](https://github.com/user-attachments/assets/fffa4237-05d2-4f77-9b5d-b437a15d464e)


## License
This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

---

This project represents a personal educational journey into further into Rust as well app design as multiple complex components had to be linked together, pushing the boundaries of my knowledge and skills.
