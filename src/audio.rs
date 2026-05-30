use anyhow::{Context, Result, anyhow, bail};
use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use ringbuf::HeapRb;
use ringbuf::traits::{Consumer, Producer, Split};

pub type SampleConsumer = ringbuf::HeapCons<f32>;

pub struct DeviceInfo {
    pub name: String,
    pub max_input_channels: u16,
}

pub struct CaptureHandle {
    pub stream: cpal::Stream,
    pub sample_rate: u32,
    pub consumer: SampleConsumer,
}

pub fn list_input_devices() -> Result<Vec<(cpal::Device, DeviceInfo)>> {
    let host = cpal::default_host();
    let mut out = Vec::new();
    let devices = host
        .input_devices()
        .context("failed to enumerate input devices")?;
    for device in devices {
        let name = device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|_| "Unknown".to_string());
        let max_input_channels = device
            .supported_input_configs()
            .ok()
            .and_then(|iter| iter.map(|c| c.channels()).max())
            .unwrap_or(0);
        if max_input_channels > 0 {
            out.push((device, DeviceInfo {
                name,
                max_input_channels,
            }));
        }
    }
    Ok(out)
}

pub fn find_device_by_name(name: &str) -> Result<Option<(cpal::Device, DeviceInfo)>> {
    let mut devices = list_input_devices()?;
    Ok(devices
        .drain(..)
        .find(|(_, info)| info.name == name))
}

pub fn pick_f32_config(
    device: &cpal::Device,
    required_channels: u16,
) -> Result<cpal::SupportedStreamConfig> {
    let configs = device
        .supported_input_configs()
        .context("failed to list supported input configs")?;
    let mut best: Option<cpal::SupportedStreamConfigRange> = None;
    for range in configs {
        if range.channels() < required_channels {
            continue;
        }
        if range.sample_format() != SampleFormat::F32 {
            continue;
        }
        let prefers_48k =
            range.min_sample_rate() <= 48_000 && range.max_sample_rate() >= 48_000;
        let prefers_441 =
            range.min_sample_rate() <= 44_100 && range.max_sample_rate() >= 44_100;
        if prefers_48k || prefers_441 || best.is_none() {
            best = Some(range.clone());
            if prefers_48k {
                break;
            }
        }
    }
    let range = best.ok_or_else(|| {
        anyhow!(
            "device does not expose an f32 input config with at least {required_channels} channel(s)"
        )
    })?;

    let target_sr: u32 = if range.min_sample_rate() <= 48_000 && range.max_sample_rate() >= 48_000 {
        48_000
    } else if range.min_sample_rate() <= 44_100 && range.max_sample_rate() >= 44_100 {
        44_100
    } else {
        range.max_sample_rate()
    };
    Ok(range.with_sample_rate(target_sr))
}

pub fn start_capture(
    device: cpal::Device,
    channel_index: u16,
    ringbuf_capacity: usize,
) -> Result<CaptureHandle> {
    let required_channels = channel_index + 1;
    let supported = pick_f32_config(&device, required_channels)?;
    let channels = supported.channels();
    if channel_index >= channels {
        bail!(
            "channel index {channel_index} out of range for device with {channels} channels"
        );
    }
    let sample_rate = supported.sample_rate();
    let stream_config = supported.config();

    let rb = HeapRb::<f32>::new(ringbuf_capacity);
    let (mut producer, consumer) = rb.split();

    let channels_usize = channels as usize;
    let channel_idx = channel_index as usize;

    let err_fn = |err| eprintln!("audio stream error: {err}");

    let stream = device
        .build_input_stream(
            &stream_config,
            move |data: &[f32], _info| {
                for frame in data.chunks_exact(channels_usize) {
                    let _ = producer.try_push(frame[channel_idx]);
                }
            },
            err_fn,
            None,
        )
        .context("failed to build input stream")?;

    stream.play().context("failed to start input stream")?;

    Ok(CaptureHandle {
        stream,
        sample_rate,
        consumer,
    })
}

pub fn drain_into(consumer: &mut SampleConsumer, dest: &mut Vec<f32>, max: usize) -> usize {
    let mut n = 0;
    while n < max {
        match consumer.try_pop() {
            Some(s) => {
                dest.push(s);
                n += 1;
            }
            None => break,
        }
    }
    n
}
