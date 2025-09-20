use eframe::egui;
use sdl2::audio::AudioQueue;
use sdl2::EventPump;

use crate::render;
use crate::sdl2_setup;
use crate::Cpu;

use std::time::Instant;

pub struct MyApp {
    label: String,
    value: f32,
    show_fps: bool,
    frame_count: i32,
    baseline: Instant,
    trace_on: bool,
    audio_device: AudioQueue<f32>,
    event_pump: EventPump,
    cpu: Cpu,
    texture: egui::TextureHandle,
}

impl MyApp {
    pub fn new(
        show_fps: bool,
        frame_count: i32,
        baseline: Instant,
        trace_on: bool,
        audio_device: AudioQueue<f32>,
        event_pump: EventPump,
        cpu: Cpu,
        cc: &eframe::CreationContext<'_>,
    ) -> Self {
        Self {
            label: "".to_string(),
            value: 0.0,
            show_fps,
            frame_count,
            baseline,
            trace_on,
            audio_device,
            event_pump,
            cpu,
            texture: cc.egui_ctx.load_texture(
                "Noise",
                egui::ColorImage::example(),
                egui::TextureOptions::NEAREST,
            ),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut new_frame = None;
        while new_frame.is_none() {
            new_frame = self.step_gb();
        }

        self.texture.set(
            egui::ColorImage {
                size: [160, 144],
                source_size: egui::Vec2 { x: 160.0, y: 144.0 },
                pixels: new_frame.unwrap().data,
            },
            egui::TextureOptions::NEAREST,
        );
        let sized_texture = egui::load::SizedTexture::new(self.texture.id(), [160.0, 144.0]);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.add(
                egui::Image::new(sized_texture)
                    .fit_to_exact_size(egui::vec2(2.0 * 160.0, 2.0 * 144.0)),
            );
            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }
            ui.label(format!("Hello '{}', value: {}", self.label, self.value));
        });

        ctx.request_repaint();
    }
}

impl MyApp {
    // Display frame if result returned is true
    fn step_gb(&mut self) -> Option<render::Frame> {
        if self.show_fps && self.frame_count == 0 {
            self.baseline = Instant::now();
        } else if self.frame_count == 30 {
            let thirty_frame_time = self.baseline.elapsed().as_secs_f32();
            self.frame_count = 1;
            self.baseline = Instant::now();
            if self.show_fps {
                let fps = 30.0 / thirty_frame_time;
                println!("FPS is {fps}");
            }
        }

        let frame = if self.trace_on {
            self.cpu.step_with_trace()
        } else {
            self.cpu.step(|_| {})
        };

        if let Some(frame) = frame {
            let frame = frame.clone();
            /*
            // present frame
            texture.update(None, &frame.data, 160 * 3).unwrap();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();
            */
            // play audio
            self.audio_device
                .queue_audio(&self.cpu.bus.audio_buffer)
                .unwrap();
            while self.audio_device.size() > 5000 {}

            // check user input
            sdl2_setup::get_user_input(&mut self.event_pump, &mut self.cpu.bus.joypad);

            // If FPS enabled, increment counter
            if self.show_fps {
                self.frame_count += 1;
            }

            return Some(frame);
        }

        None
    }
}
