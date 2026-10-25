use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use glam::Vec4;
use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};

const FFT_SIZE: usize = 4096;
const BANDS: usize = 64;
const F_MIN: f32 = 40.0;
const F_MAX: f32 = 16_000.0;

pub struct Audio {
    _stream: cpal::Stream,
    samples: Arc<Mutex<VecDeque<f32>>>,
    pushed: Arc<AtomicUsize>,
    last_pushed: usize,
    sample_rate: f32,
    fft: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    input: Vec<f32>,
    output: Vec<Complex<f32>>,
    bands: [f32; BANDS],
}

impl Audio {
    pub fn new() -> anyhow::Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("нет устройства вывода звука"))?;
        let config = device.default_output_config()?;
        let channels = config.channels() as usize;
        let sample_rate = config.sample_rate() as f32;

        let samples = Arc::new(Mutex::new(VecDeque::with_capacity(FFT_SIZE + 1024)));
        let pushed = Arc::new(AtomicUsize::new(0));

        let (cb_samples, cb_pushed) = (samples.clone(), pushed.clone());
        let stream_config: cpal::StreamConfig = config.clone().into();

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                stream_config,
                move |data: &[f32], _| {
                    let mut q = cb_samples.lock().unwrap();
                    for frame in data.chunks(channels) {
                        q.push_back(frame.iter().sum::<f32>() / channels as f32);
                    }
                    while q.len() > FFT_SIZE {
                        q.pop_front();
                    }
                    cb_pushed.fetch_add(data.len(), Ordering::Relaxed);
                },
                |e| eprintln!("audio error: {e}"),
                None,
            )?,
            other => anyhow::bail!("формат сэмплов {other:?} пока не поддержан"),
        };
        stream.play()?;

        let fft = RealFftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
        let output = fft.make_output_vec();
        let window = (0..FFT_SIZE)
            .map(|i| {
                0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (FFT_SIZE - 1) as f32).cos()
            })
            .collect();

        Ok(Self {
            _stream: stream,
            samples,
            pushed,
            last_pushed: 0,
            sample_rate,
            fft,
            window,
            input: vec![0.0; FFT_SIZE],
            output,
            bands: [0.0; BANDS],
        })
    }

    pub fn update(&mut self) -> [Vec4; 16] {
        let pushed = self.pushed.load(Ordering::Relaxed);
        let active = pushed != self.last_pushed;
        self.last_pushed = pushed;

        let mut target = [0.0f32; BANDS];

        if active {
            {
                let q = self.samples.lock().unwrap();
                self.input.fill(0.0);
                let skip = FFT_SIZE - q.len();
                for (i, s) in q.iter().enumerate() {
                    self.input[skip + i] = *s * self.window[skip + i];
                }
            }
            self.fft.process(&mut self.input, &mut self.output).unwrap();

            let bin_hz = self.sample_rate / FFT_SIZE as f32;
            let edge = |b: usize| F_MIN * (F_MAX / F_MIN).powf(b as f32 / BANDS as f32);

            for b in 0..BANDS {
                let lo = ((edge(b) / bin_hz).floor() as usize).max(1);
                let hi = ((edge(b + 1) / bin_hz).ceil() as usize)
                    .max(lo + 1)
                    .min(FFT_SIZE / 2);

                let mut peak = 0.0f32;
                for k in lo..hi {
                    peak = peak.max(self.output[k].norm() / (FFT_SIZE as f32 / 4.0));
                }

                let db = 20.0 * peak.max(1e-5).log10() + 12.0 * (b as f32 / BANDS as f32);
                target[b] = ((db + 70.0) / 70.0).clamp(0.0, 1.0);
            }
        }

        for b in 0..BANDS {
            let (old, new) = (self.bands[b], target[b]);
            self.bands[b] = if new > old {
                old * 0.4 + new * 0.6
            } else {
                old * 0.88 + new * 0.12
            };
        }

        let mut out = [Vec4::ZERO; 16];
        for b in 0..BANDS {
            out[b / 4][b % 4] = self.bands[b];
        }
        out
    }
}
