pub mod app;
pub mod audio;
pub mod config;
pub mod error;
pub mod music;
pub mod pitch;
pub mod ui;

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender, bounded};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::app::{App, Transition};
use crate::audio::{CaptureHandle, SampleConsumer, drain_into, list_input_devices, start_capture};
use crate::pitch::{PitchDetector, PitchEvent, WINDOW_SIZE, YinDetector};

const RINGBUF_CAPACITY: usize = WINDOW_SIZE * 4;
const RENDER_TICK: Duration = Duration::from_millis(33);

pub fn run() -> Result<()> {
    let devices = list_input_devices().context("listing audio input devices")?;
    let config = config::load();

    let mut terminal = ratatui::init();
    let res = run_app(&mut terminal, devices, config);
    ratatui::restore();
    res
}

fn run_app(
    terminal: &mut ratatui::DefaultTerminal,
    devices: Vec<(cpal::Device, audio::DeviceInfo)>,
    config: config::StoredConfig,
) -> Result<()> {
    let mut app = App::new(devices, config);
    let mut active: Option<ActiveCapture> = None;

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;

        if let Some(active) = &active {
            while let Ok(event) = active.events.try_recv() {
                app.on_pitch_event(event);
            }
        }
        app.tick();

        if crossterm::event::poll(RENDER_TICK)? {
            match crossterm::event::read()? {
                crossterm::event::Event::Key(key) => {
                    if key.kind != crossterm::event::KeyEventKind::Release {
                        match app.handle_key(key) {
                            Transition::Stay => {}
                            Transition::Quit => {
                                if let Some(active) = active.take() {
                                    active.shutdown();
                                }
                                let _ = config::save(&app.config);
                                return Ok(());
                            }
                            Transition::StartCapture {
                                device,
                                device_name,
                                channel,
                                key,
                            } => {
                                let _ = config::save(&app.config);
                                if let Some(prev) = active.take() {
                                    prev.shutdown();
                                }
                                match spawn_capture(device, channel) {
                                    Ok(capture) => {
                                        active = Some(capture);
                                        app.enter_drill(device_name, channel, key);
                                    }
                                    Err(e) => {
                                        app.status =
                                            Some(format!("audio start failed: {e:#}"));
                                        app.reset_to_device_picker();
                                    }
                                }
                            }
                        }
                    }
                }
                crossterm::event::Event::Resize(_, _) => {}
                _ => {}
            }
        }
    }
}

struct ActiveCapture {
    _stream: cpal::Stream,
    events: Receiver<PitchEvent>,
    shutdown: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl ActiveCapture {
    fn shutdown(mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

fn spawn_capture(device: cpal::Device, channel: u16) -> Result<ActiveCapture> {
    let CaptureHandle {
        stream,
        sample_rate,
        consumer,
    } = start_capture(device, channel, RINGBUF_CAPACITY)?;
    let (tx, rx) = bounded::<PitchEvent>(16);
    let shutdown = Arc::new(AtomicBool::new(false));
    let worker = spawn_pitch_worker(consumer, sample_rate, tx, shutdown.clone());

    Ok(ActiveCapture {
        _stream: stream,
        events: rx,
        shutdown,
        worker: Some(worker),
    })
}

fn spawn_pitch_worker(
    mut consumer: SampleConsumer,
    sample_rate: u32,
    tx: Sender<PitchEvent>,
    shutdown: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut detector = YinDetector::new(WINDOW_SIZE);
        let mut buf: Vec<f32> = Vec::with_capacity(WINDOW_SIZE * 2);
        while !shutdown.load(Ordering::Relaxed) {
            drain_into(&mut consumer, &mut buf, WINDOW_SIZE);
            if buf.len() >= WINDOW_SIZE {
                let start = buf.len() - WINDOW_SIZE;
                let window = &buf[start..];
                if let Some(evt) = detector.detect(window, sample_rate) {
                    if tx.send(evt).is_err() {
                        break;
                    }
                }
                let keep = WINDOW_SIZE / 2;
                let drop_count = buf.len() - keep;
                buf.drain(..drop_count);
            } else {
                thread::sleep(Duration::from_millis(5));
            }
        }
    })
}
