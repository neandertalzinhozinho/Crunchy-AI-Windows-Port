use anyhow::{bail, Context, Result};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, SupportedStreamConfig,
};
use crossbeam_queue::ArrayQueue;
use hushmic_denoiser::{init_runtime, Denoiser, Mode, HOP, SAMPLE_RATE};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const CABLE_VENDOR_NAME: &str = "vb-audio";

pub fn devices() -> Result<Vec<String>> {
    let mut names = Vec::new();
    for device in cpal::default_host().input_devices()? {
        let name = device.name()?;
        // Do not let the VB-CABLE recording endpoint feed processed audio back in.
        if !is_cable_recording_name(&name) && !name.to_lowercase().contains("hushmic") {
            names.push(name);
        }
    }
    Ok(names)
}

pub fn driver_status() -> (bool, String) {
    match (virtual_output_device(), virtual_input_device()) {
        (Ok(output), Ok(input)) => {
            let output_name = output.name().unwrap_or_else(|_| "CABLE Input".into());
            let input_name = input.name().unwrap_or_else(|_| "CABLE Output".into());
            (true, format!("VB-CABLE pronto. Saída HushMic: {output_name}. Microfone para Discord/OBS: {input_name}."))
        }
        (Err(error), _) | (_, Err(error)) => (
            false,
            format!("VB-CABLE não encontrado. Instale o driver VB-CABLE e clique em Verificar driver. Detalhe: {error}"),
        ),
    }
}

fn is_cable_recording_name(name: &str) -> bool {
    let name = name.to_lowercase();
    name.contains("cable output") && name.contains(CABLE_VENDOR_NAME)
}

fn is_cable_playback_name(name: &str) -> bool {
    let name = name.to_lowercase();
    name.contains("cable input") && name.contains(CABLE_VENDOR_NAME)
}

fn is_monitor_candidate(name: &str) -> bool {
    !is_cable_playback_name(name)
        && !is_cable_recording_name(name)
        && !name.to_lowercase().contains("hushmic")
}

fn virtual_output_device() -> Result<cpal::Device> {
    cpal::default_host()
        .output_devices()?
        .find(|device| {
            device
                .name()
                .is_ok_and(|name| is_cable_playback_name(&name))
        })
        .context("endpoint de reprodução CABLE Input não enumerado pelo Windows")
}

fn virtual_input_device() -> Result<cpal::Device> {
    cpal::default_host()
        .input_devices()?
        .find(|device| {
            device
                .name()
                .is_ok_and(|name| is_cable_recording_name(&name))
        })
        .context("endpoint de gravação CABLE Output não enumerado pelo Windows")
}

pub struct Control {
    pub mode: AtomicUsize,
    pub strength: AtomicU32,
    pub peak: AtomicU32,
    pub micros: AtomicU64,
    pub dropped: AtomicU64,
    pub underruns: AtomicU64,
    pub monitor: AtomicBool,
    stop: AtomicBool,
}

pub struct Session {
    pub control: Arc<Control>,
    pub errors: mpsc::Receiver<String>,
    pub monitor_errors: mpsc::Receiver<String>,
    worker: Option<JoinHandle<()>>,
}

impl Session {
    pub fn start(
        name: String,
        assets: PathBuf,
        light: bool,
        mode: usize,
        strength: u32,
    ) -> Result<Self> {
        let output_device = virtual_output_device().context(
            "Microfone virtual indisponível. Instale o driver VB-CABLE antes de iniciar",
        )?;
        virtual_input_device().context("A entrada virtual CABLE Output não está disponível")?;
        let control = Arc::new(Control {
            mode: AtomicUsize::new(mode),
            strength: AtomicU32::new(strength),
            peak: AtomicU32::new(0),
            micros: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            underruns: AtomicU64::new(0),
            monitor: AtomicBool::new(false),
            stop: AtomicBool::new(false),
        });
        let (tx, errors) = mpsc::channel();
        let (monitor_tx, monitor_errors) = mpsc::channel();
        let worker_control = control.clone();
        let worker = thread::Builder::new()
            .name("hushmic-audio".into())
            .spawn(move || {
                if let Err(e) = run(
                    name,
                    assets,
                    light,
                    output_device,
                    worker_control,
                    tx.clone(),
                    monitor_tx,
                ) {
                    let _ = tx.send(format!("{e:#}"));
                }
            })?;
        Ok(Self {
            control,
            errors,
            monitor_errors,
            worker: Some(worker),
        })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.control.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn denoiser(assets: &Path, light: bool) -> Result<Denoiser> {
    init_runtime(assets.join("onnxruntime.dll"))?;
    let model = if light {
        "dpdfnet2_48khz_hr.onnx"
    } else {
        "dpdfnet8_48khz_hr.onnx"
    };
    Denoiser::from_file(assets.join(model))
        .with_context(|| format!("Não foi possível carregar {model}"))
}

fn capture<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    input_sample_rate: u32,
    queue: Arc<ArrayQueue<f32>>,
    control: Arc<Control>,
    errors: mpsc::Sender<String>,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let channels = config.channels as usize;
    let mut resampler = LinearResampler::new(input_sample_rate, SAMPLE_RATE);
    Ok(device.build_input_stream(
        config,
        move |data: &[T], _| {
            let mut peak = 0.0_f32;
            let mut emit = |sample: f32| {
                peak = peak.max(sample.abs());
                if queue.push(sample).is_err() {
                    control.dropped.fetch_add(1, Ordering::Relaxed);
                }
            };
            for frame in data.chunks_exact(channels) {
                let sample = frame
                    .iter()
                    .map(|s| <f32 as cpal::FromSample<T>>::from_sample_(*s))
                    .sum::<f32>()
                    / channels as f32;
                let sample = if sample.is_finite() {
                    sample.clamp(-1.0, 1.0)
                } else {
                    0.0
                };
                resampler.push(sample, &mut emit);
            }
            control.peak.store(peak.to_bits(), Ordering::Relaxed);
        },
        move |e| {
            let _ = errors.send(format!("Microfone desconectado ou falha de áudio: {e}"));
        },
        None,
    )?)
}

/// Chooses the closest supported input rate to the denoiser's 48 kHz rate.
/// Resampling is performed in the capture callback so common 44.1 kHz devices work.
fn select_input_config(device: &cpal::Device) -> Result<SupportedStreamConfig> {
    let best = device
        .supported_input_configs()?
        .filter(|c| {
            matches!(
                c.sample_format(),
                cpal::SampleFormat::F32 | cpal::SampleFormat::I16 | cpal::SampleFormat::I32
            )
        })
        .min_by_key(|c| {
            let rate = SAMPLE_RATE.clamp(c.min_sample_rate().0, c.max_sample_rate().0);
            (rate.abs_diff(SAMPLE_RATE), c.channels())
        })
        .context("O microfone não oferece um formato de áudio compatível")?;
    let rate = SAMPLE_RATE.clamp(best.min_sample_rate().0, best.max_sample_rate().0);
    Ok(best.with_sample_rate(cpal::SampleRate(rate)))
}

fn render<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    queue: Arc<ArrayQueue<f32>>,
    control: Arc<Control>,
    errors: mpsc::Sender<String>,
) -> Result<cpal::Stream>
where
    T: cpal::SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    Ok(device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            for frame in data.chunks_mut(channels) {
                let sample = match queue.pop() {
                    Some(sample) => sample,
                    None => {
                        control.underruns.fetch_add(1, Ordering::Relaxed);
                        0.0
                    }
                };
                let sample = T::from_sample(sample);
                frame.fill(sample);
            }
        },
        move |e| {
            let _ = errors.send(format!("Falha no microfone virtual VB-CABLE: {e}"));
        },
        None,
    )?)
}

/// Streaming linear resampler with no allocation, suitable for the CPAL callback.
struct LinearResampler {
    input_per_output: f64,
    previous: Option<f32>,
    input_index: u64,
    next_output_at: f64,
}

impl LinearResampler {
    fn new(input_rate: u32, output_rate: u32) -> Self {
        let input_per_output = input_rate as f64 / output_rate.max(1) as f64;
        Self {
            input_per_output,
            previous: None,
            input_index: 0,
            next_output_at: input_per_output,
        }
    }

    fn push(&mut self, sample: f32, emit: &mut impl FnMut(f32)) {
        let Some(previous) = self.previous else {
            self.previous = Some(sample);
            return;
        };
        let left = self.input_index;
        let right = left + 1;
        while self.next_output_at <= right as f64 {
            let fraction = (self.next_output_at - left as f64).clamp(0.0, 1.0) as f32;
            emit(previous + (sample - previous) * fraction);
            self.next_output_at += self.input_per_output;
        }
        self.previous = Some(sample);
        self.input_index = right;
    }
}

fn run(
    name: String,
    assets: PathBuf,
    light: bool,
    output_device: cpal::Device,
    control: Arc<Control>,
    errors: mpsc::Sender<String>,
    monitor_errors: mpsc::Sender<String>,
) -> Result<()> {
    let mut denoiser = denoiser(&assets, light)?;
    let output_config = output_device
        .supported_output_configs()?
        .filter(|c| c.min_sample_rate().0 <= SAMPLE_RATE && c.max_sample_rate().0 >= SAMPLE_RATE)
        .filter(|c| {
            matches!(
                c.sample_format(),
                cpal::SampleFormat::F32 | cpal::SampleFormat::I16 | cpal::SampleFormat::I32
            )
        })
        .min_by_key(|c| c.channels())
        .context("VB-CABLE não oferece um formato compatível em 48000 Hz")?
        .with_sample_rate(cpal::SampleRate(SAMPLE_RATE));
    let output_stream_config = output_config.config();
    let output_queue = Arc::new(ArrayQueue::new(4800)); // bounded to 100 ms
    let output_stream = match output_config.sample_format() {
        cpal::SampleFormat::F32 => render::<f32>(
            &output_device,
            &output_stream_config,
            output_queue.clone(),
            control.clone(),
            errors.clone(),
        )?,
        cpal::SampleFormat::I16 => render::<i16>(
            &output_device,
            &output_stream_config,
            output_queue.clone(),
            control.clone(),
            errors.clone(),
        )?,
        cpal::SampleFormat::I32 => render::<i32>(
            &output_device,
            &output_stream_config,
            output_queue.clone(),
            control.clone(),
            errors.clone(),
        )?,
        _ => bail!("Formato de saída virtual não suportado"),
    };
    let mut _monitor_stream: Option<cpal::Stream> = None;
    let mut monitor_queue: Option<Arc<ArrayQueue<f32>>> = None;
    let mut monitor_resampler: Option<LinearResampler> = None;
    let mut was_monitoring = false;
    let device = cpal::default_host()
        .input_devices()?
        .find(|d| d.name().ok().as_ref() == Some(&name))
        .context("Microfone não encontrado. Atualize a lista de dispositivos")?;
    let supported = select_input_config(&device)?;
    let input_sample_rate = supported.sample_rate().0;
    let queue = Arc::new(ArrayQueue::new(4800)); // bounded to 100 ms
    let config = supported.config();
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => capture::<f32>(
            &device,
            &config,
            input_sample_rate,
            queue.clone(),
            control.clone(),
            errors,
        )?,
        cpal::SampleFormat::I16 => capture::<i16>(
            &device,
            &config,
            input_sample_rate,
            queue.clone(),
            control.clone(),
            errors,
        )?,
        cpal::SampleFormat::I32 => capture::<i32>(
            &device,
            &config,
            input_sample_rate,
            queue.clone(),
            control.clone(),
            errors,
        )?,
        _ => bail!("Formato de áudio não suportado"),
    };
    output_stream.play()?;
    stream.play()?;
    let mut input = [0.0; HOP];
    let mut output = [0.0; HOP];
    let mut last_input = Instant::now();
    while !control.stop.load(Ordering::Relaxed) {
        let monitoring = control.monitor.load(Ordering::Relaxed);
        if monitoring && !was_monitoring {
            match start_monitor(&control, monitor_errors.clone()) {
                Ok((stream, samples, resampler)) => {
                    _monitor_stream = Some(stream);
                    monitor_queue = Some(samples);
                    monitor_resampler = Some(resampler);
                }
                Err(error) => {
                    control.monitor.store(false, Ordering::Relaxed);
                    let _ = monitor_errors.send(format!(
                        "Não foi possível iniciar o monitoramento local: {error:#}"
                    ));
                }
            }
        } else if !monitoring && was_monitoring {
            _monitor_stream = None;
            monitor_queue = None;
            monitor_resampler = None;
        }
        was_monitoring = control.monitor.load(Ordering::Relaxed);
        if queue.len() < HOP {
            if last_input.elapsed() > Duration::from_secs(3) {
                bail!("O microfone parou de fornecer áudio. Reinicie a captura.");
            }
            thread::sleep(Duration::from_millis(1));
            continue;
        }
        last_input = Instant::now();
        // Drop an accumulated backlog after a scheduler stall instead of replaying old speech.
        if queue.len() > HOP * 4 {
            while queue.len() > HOP * 2 {
                queue.pop();
                control.dropped.fetch_add(1, Ordering::Relaxed);
            }
            denoiser.reset();
        }
        for s in &mut input {
            *s = queue.pop().unwrap_or(0.0);
        }
        denoiser.set_mode(match control.mode.load(Ordering::Relaxed) {
            1 => Mode::Bypass,
            2 => Mode::Mute,
            _ => Mode::Process,
        });
        denoiser.set_attenuation_limit_db(control.strength.load(Ordering::Relaxed) as f32);
        let start = Instant::now();
        denoiser.process_hop(&input, &mut output)?;
        control
            .micros
            .store(start.elapsed().as_micros() as u64, Ordering::Relaxed);
        for sample in &output {
            let sample = sample.clamp(-1.0, 1.0);
            if output_queue.push(sample).is_err() {
                // Keep fresh speech if the render callback falls behind.
                output_queue.pop();
                let _ = output_queue.push(sample);
                control.dropped.fetch_add(1, Ordering::Relaxed);
            }
            if let (Some(samples), Some(resampler)) = (&monitor_queue, &mut monitor_resampler) {
                resampler.push(sample, &mut |s| {
                    if samples.push(s).is_err() {
                        samples.pop();
                        let _ = samples.push(s);
                    }
                });
            }
        }
    }
    drop(stream);
    drop(output_stream);
    // Closing the render stream leaves the virtual microphone silent.
    Ok(())
}

fn start_monitor(
    control: &Arc<Control>,
    errors: mpsc::Sender<String>,
) -> Result<(cpal::Stream, Arc<ArrayQueue<f32>>, LinearResampler)> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .context("O Windows não tem uma saída de áudio padrão")?;
    let name = device.name().unwrap_or_default();
    anyhow::ensure!(is_monitor_candidate(&name), "A saída padrão é o próprio VB-CABLE/HushMic; selecione seus fones como saída padrão do Windows");
    let supported = device
        .supported_output_configs()?
        .filter(|c| {
            matches!(
                c.sample_format(),
                cpal::SampleFormat::F32 | cpal::SampleFormat::I16 | cpal::SampleFormat::I32
            )
        })
        .min_by_key(|c| {
            let rate = SAMPLE_RATE.clamp(c.min_sample_rate().0, c.max_sample_rate().0);
            (rate.abs_diff(SAMPLE_RATE), c.channels())
        })
        .context("A saída padrão não oferece formato de áudio compatível")?;
    let rate = SAMPLE_RATE.clamp(supported.min_sample_rate().0, supported.max_sample_rate().0);
    let queue = Arc::new(ArrayQueue::new(rate as usize / 10));
    let config = supported.with_sample_rate(cpal::SampleRate(rate)).config();
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => render::<f32>(
            &device,
            &config,
            queue.clone(),
            control.clone(),
            errors.clone(),
        )?,
        cpal::SampleFormat::I16 => render::<i16>(
            &device,
            &config,
            queue.clone(),
            control.clone(),
            errors.clone(),
        )?,
        cpal::SampleFormat::I32 => {
            render::<i32>(&device, &config, queue.clone(), control.clone(), errors)?
        }
        _ => bail!("Formato de monitoramento não suportado"),
    };
    stream
        .play()
        .context("Não foi possível iniciar o áudio de monitoramento")?;
    Ok((stream, queue, LinearResampler::new(SAMPLE_RATE, rate)))
}

pub fn self_test(assets: &Path) -> Result<()> {
    for light in [false, true] {
        let mut d = denoiser(assets, light)?;
        let mut input = [0.0; HOP];
        let mut output = [0.0; HOP];
        let start = Instant::now();
        for frame in 0..100 {
            for (i, s) in input.iter_mut().enumerate() {
                *s = ((frame * HOP + i) as f32 * 0.0576).sin() * 0.1;
            }
            d.process_hop(&input, &mut output)?;
            anyhow::ensure!(
                output.iter().all(|s| s.is_finite()),
                "Non-finite model output"
            );
        }
        println!(
            "{}: 1 s de áudio processado em {:.1} ms; latência do motor: {} amostras",
            if light { "Leve" } else { "Qualidade" },
            start.elapsed().as_secs_f64() * 1000.0,
            d.latency_samples()
        );
        d.set_mode(Mode::Mute);
        d.process_hop(&input, &mut output)?;
        d.process_hop(&input, &mut output)?;
        anyhow::ensure!(
            output.iter().all(|s| *s == 0.0),
            "Mute must produce digital silence"
        );
        println!("Mudo: silêncio digital confirmado");
    }
    Ok(())
}

#[cfg(test)]
mod resampler_tests {
    use super::LinearResampler;

    fn resample_count(input_rate: u32, output_rate: u32, frames: usize) -> usize {
        let mut resampler = LinearResampler::new(input_rate, output_rate);
        let mut count = 0;
        for _ in 0..frames {
            resampler.push(0.25, &mut |_| count += 1);
        }
        count
    }

    #[test]
    fn converts_44100_to_48000_without_drifting() {
        let count = resample_count(44_100, 48_000, 44_100);
        assert!((47_998..=48_001).contains(&count), "got {count}");
    }

    #[test]
    fn converts_96000_to_48000() {
        let count = resample_count(96_000, 48_000, 96_000);
        assert!((47_998..=48_001).contains(&count), "got {count}");
    }

    #[test]
    fn preserves_constant_signal() {
        let mut resampler = LinearResampler::new(44_100, 48_000);
        let mut output = Vec::new();
        for _ in 0..100 {
            resampler.push(0.25, &mut |sample| output.push(sample));
        }
        assert!(!output.is_empty());
        assert!(output
            .iter()
            .all(|sample| (*sample - 0.25).abs() < f32::EPSILON));
    }

    #[test]
    fn matches_vb_cable_endpoints_by_audio_direction() {
        assert!(super::is_cable_playback_name(
            "CABLE Input (VB-Audio Virtual Cable)"
        ));
        assert!(!super::is_cable_playback_name(
            "CABLE Output (VB-Audio Virtual Cable)"
        ));
        assert!(super::is_cable_recording_name(
            "CABLE Output (VB-Audio Virtual Cable)"
        ));
        assert!(!super::is_cable_recording_name(
            "CABLE Input (VB-Audio Virtual Cable)"
        ));
    }

    #[test]
    fn monitor_rejects_virtual_endpoints_but_accepts_headphones() {
        assert!(super::is_monitor_candidate("Headphones (USB Audio)"));
        assert!(!super::is_monitor_candidate(
            "CABLE Input (VB-Audio Virtual Cable)"
        ));
        assert!(!super::is_monitor_candidate(
            "CABLE Output (VB-Audio Virtual Cable)"
        ));
        assert!(!super::is_monitor_candidate("HushMic Virtual Output"));
    }
}
