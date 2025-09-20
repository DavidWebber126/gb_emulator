use eframe::egui::{self, Event};
use sdl2::audio::AudioQueue;

use lazy_static::lazy_static;

use crate::ppu::ScreenOptions;
use crate::render;
use crate::Cpu;

use std::collections::HashMap;
use std::time::Instant;

pub struct MyApp {
    side_panel: SidePanel,
    paused: bool,
    show_fps: bool,
    frame_count: i32,
    baseline: Instant,
    trace_on: bool,
    audio_device: AudioQueue<f32>,
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
        cpu: Cpu,
        cc: &eframe::CreationContext<'_>,
    ) -> Self {
        Self {
            side_panel: SidePanel::Cpu,
            paused: false,
            show_fps,
            frame_count,
            baseline,
            trace_on,
            audio_device,
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
        // Step CPU and capture latest frame
        let mut new_frame = None;
        while new_frame.is_none() && !self.paused {
            new_frame = self.step_gb();
        }

        if self.paused {
            new_frame = Some(self.cpu.bus.last_frame.clone());
        };

        ctx.input(|i| {
            for event in &i.events {
                match event {
                    Event::Key {
                        key: egui::Key::Escape,
                        ..
                    } => std::process::exit(0),
                    // Pause Emulation
                    Event::Key {
                        key: egui::Key::P,
                        pressed: true,
                        ..
                    } => {
                        self.paused = !self.paused;
                    }
                    // Step CPU by one
                    Event::Key {
                        key: egui::Key::F,
                        pressed: true,
                        ..
                    } => {
                        if self.paused {
                            self.step_gb();
                            new_frame = Some(self.cpu.bus.last_frame.clone());
                        }
                    }
                    Event::Key {
                        pressed: true, key, ..
                    } => {
                        if let Some(&(mode, button)) = KEY_MAP.get(&key) {
                            self.cpu
                                .bus
                                .joypad
                                .button_pressed_status(mode, button, true);
                        }
                    }
                    Event::Key {
                        pressed: false,
                        key,
                        ..
                    } => {
                        if let Some(&(mode, button)) = KEY_MAP.get(&key) {
                            self.cpu
                                .bus
                                .joypad
                                .button_pressed_status(mode, button, false);
                        }
                    }
                    _ => {}
                }
            }
        });

        self.texture.set(
            egui::ColorImage {
                size: [160, 144],
                source_size: egui::Vec2 { x: 160.0, y: 144.0 },
                pixels: new_frame.unwrap().data,
            },
            egui::TextureOptions::NEAREST,
        );
        let sized_texture = egui::load::SizedTexture::new(self.texture.id(), [160.0, 144.0]);

        // UI Layout

        // Side Panel
        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(400.0)
            .width_range(500.0..=1200.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut self.side_panel, SidePanel::Cpu, "CPU");
                        ui.selectable_value(&mut self.side_panel, SidePanel::Ppu, "PPU");
                        ui.selectable_value(&mut self.side_panel, SidePanel::Apu, "APU");
                    })
                });

                match self.side_panel {
                    SidePanel::Cpu => {
                        for string in &self.cpu.prev_instrs {
                            ui.add(egui::Label::new(string));
                        }
                    }
                    SidePanel::Ppu => {
                        ui.add(egui::Label::new(
                            "This is where PPU tile map, window and sprites will go",
                        ));
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut self.cpu.bus.ppu.screen_options,
                                ScreenOptions::All,
                                "Normal",
                            );
                            ui.selectable_value(
                                &mut self.cpu.bus.ppu.screen_options,
                                ScreenOptions::BackgroundOnly,
                                "Background",
                            );
                            ui.selectable_value(
                                &mut self.cpu.bus.ppu.screen_options,
                                ScreenOptions::WindowOnly,
                                "Window",
                            );
                            ui.selectable_value(
                                &mut self.cpu.bus.ppu.screen_options,
                                ScreenOptions::SpritesOnly,
                                "Sprites",
                            );
                        });
                    }
                    SidePanel::Apu => {
                        ui.add(egui::Label::new("This is where audio waves will go"));
                    }
                }
            });

        // Central Panel
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(egui::Image::new(sized_texture)
                .fit_to_exact_size(egui::vec2(3.0 * 160.0, 3.0 * 144.0)),
            );

            ui.heading("Current CPU State");

            let cpu_state = format!(
                "A: {:02X}   F: {:02X}   B: {:02X}   C: {:02X}   D: {:02X}   E: {:02X}   H: {:02X}   L: {:02X}\nStack Pointer: {:04X}   Program Counter: {:04X}\nIME: {}   IE: {:08b}   IF: {:08b}",
                self.cpu.a,
                self.cpu.flags.bits(),
                self.cpu.b,
                self.cpu.c,
                self.cpu.d,
                self.cpu.e,
                self.cpu.h,
                self.cpu.l,
                self.cpu.stack_pointer,
                self.cpu.program_counter,
                self.cpu.ime,
                self.cpu.bus.interrupt_enable,
                self.cpu.bus.interrupt_flag,
            );

            ui.add(egui::Label::new(cpu_state));
            // ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            // if ui.button("Increment").clicked() {
            //     self.value += 1.0;
            // }
            // ui.label(format!("Hello '{}', value: {}", self.label, self.value));
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
            //sdl2_setup::get_user_input(&mut self.event_pump, &mut self.cpu.bus.joypad);

            // If FPS enabled, increment counter
            if self.show_fps {
                self.frame_count += 1;
            }

            return Some(frame);
        }

        None
    }
}

lazy_static! {
    static ref KEY_MAP: HashMap<egui::Key, (bool, u8)> = {
        let mut key_map = HashMap::new();

        // true = select mode, false = dpad mode
        key_map.insert(egui::Key::ArrowDown, (false, 0b0000_1000));
        key_map.insert(egui::Key::ArrowUp, (false, 0b0000_0100));
        key_map.insert(egui::Key::ArrowLeft, (false, 0b0000_0010));
        key_map.insert(egui::Key::ArrowRight, (false, 0b0000_0001));
        key_map.insert(egui::Key::Enter, (true, 0b0000_1000));
        key_map.insert(egui::Key::Space, (true, 0b0000_0100));
        key_map.insert(egui::Key::S, (true, 0b0000_0010));
        key_map.insert(egui::Key::A, (true, 0b0000_0001));

        key_map
    };
}

#[derive(Debug, PartialEq)]
enum SidePanel {
    Cpu,
    Ppu,
    Apu,
}
