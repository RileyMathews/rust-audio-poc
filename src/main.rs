use portaudio;
use std::sync::mpsc::*;

fn main() {
  // Construct a portaudio instance that will connect to a native audio API
  let pa = portaudio::PortAudio::new().expect("Unable to init PortAudio");
  // Collect information about the default microphone
  let mic_index = pa
    .default_input_device()
    .expect("Unable to get default device");
  let mic = pa.device_info(mic_index).expect("unable to get mic info");

  // Set parameters for the stream settings.
  // We pass which mic should be used, how many channels are used,
  // whether all the values of all the channels should be passed in a
  // single audiobuffer and the latency that should be considered
  let input_params =
    portaudio::StreamParameters::<f32>::new(mic_index, 1, true, mic.default_low_input_latency);
  println!("{:?}", mic.default_sample_rate);
  // Settings for an inputstream.
  // Here we pass the stream parameters we set before,
  // the sample rate of the mic and the amount values we want to receive
  let input_settings =
    portaudio::InputStreamSettings::new(input_params, mic.default_sample_rate, 256);

  // Creating a channel so we can receive audio values asynchronously
  let (sender, receiver) = channel();

  // A callback function that should be as short as possible so we send all the info to a different thread
  let callback = move |portaudio::InputStreamCallbackArgs { buffer, .. }| match sender.send(buffer)
  {
    Ok(_) => portaudio::Continue,
    Err(_) => portaudio::Complete,
  };

  // Creating & starting the input stream with our settings & callback
  let mut stream = pa
    .open_non_blocking_stream(input_settings, callback)
    .expect("Unable to create stream");
  stream.start().expect("Unable to start stream");

  // Printing values every time we receive new ones while the stream is active
  while stream.is_active().unwrap() {
    while let Ok(buffer) = receiver.try_recv() {
      let cor = correlation(buffer);
      let peak = get_peak(cor);
      let letter = hz_to_pitch(peak);
      println!("{:?}", letter)
    }
  }
}

fn get_peak(cor: Vec<f32>) -> f32 {
  let first_peak_end = match cor.iter().position(|&c| c < 0.0) {
    Some(p) => p,
    None => {
      // Musical signals will drop below 0 at some point.
      // This exits the whole function early with a result of
      // 'no fundamental frequency' if it doesn't
      return 0 as f32;
    }
  };

  let peak = cor
    .iter()
    .enumerate() // Adds the indexes to the iterator
    .skip(first_peak_end) // Skips the first peak
    .fold((first_peak_end, 0.0), |(xi, xmag), (yi, &ymag)| {
      if ymag > xmag {
        (yi, ymag)
      } else {
        (xi, xmag)
      }
    });

  // The fold above returns the index and its value as a tuple. This
  // will pull out just the index and ignore the value.
  let (peak_index, _) = peak;
  48000.0 / peak_index as f32
}

fn hz_to_midi_number(hz: f32) -> f32 {
    69.0 + 12.0 * (hz / 440.0).log2()
}

fn hz_to_pitch(hz: f32) -> String {
    let pitch_names = [
        "C","C♯","D","E♭","E","F","F♯","G","G♯","A","B♭","B"
    ];

    let midi_number = hz_to_midi_number(hz);
    let rounded_pitch = midi_number.round() as i32;

    let name_index = rounded_pitch as usize % pitch_names.len();
    let name = pitch_names[name_index];
    let octave = rounded_pitch / pitch_names.len() as i32 - 1;

    // fun fact, this format string will be type checked at compile
    // time.
    format!("{: <2}{}", name, octave)
}

fn correlation(signal: &[f32]) -> Vec<f32> {
  (0..signal.len())
    .map(|offset| {
      signal
        .iter()
        .take(signal.len() - offset)
        .zip(signal.iter().skip(offset))
        .map(|(sig_i, sig_j)| sig_i * sig_j)
        .sum()
    })
    .collect()
}
