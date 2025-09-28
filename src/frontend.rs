use eframe::egui::{self, Event};
use egui_plot::{Line, Plot, PlotPoints};
use sdl2::audio::AudioQueue;

use lazy_static::lazy_static;

use crate::apu;
use crate::render;
use crate::Cpu;

use std::collections::HashMap;
use std::time::Instant;
use std::{fs, path::PathBuf};

pub struct GameSelect<'a> {
    filepaths: Vec<PathBuf>,
    selected_item: Option<PathBuf>,
    selected_game: &'a mut Option<PathBuf>,
}

impl<'a> GameSelect<'a> {
    pub fn new(selected_game: &'a mut Option<PathBuf>) -> Self {
        let paths = fs::read_dir("roms/games/").unwrap();
        let mut filepaths = Vec::new();
        for path in paths {
            filepaths.push(path.unwrap().path());
        }
        Self {
            filepaths: filepaths,
            selected_item: None,
            selected_game,
        }
    }
}

impl eframe::App for GameSelect<'_> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.selected_item.is_none() {
                egui::ComboBox::from_label("Select a Game: ").show_ui(ui, |ui| {
                    for file in &self.filepaths {
                        ui.selectable_value(
                            &mut self.selected_item,
                            Some(file.clone()),
                            file.to_string_lossy().strip_prefix("roms/games/").unwrap(),
                        );
                    }
                });
            } else {
                *self.selected_game = self.selected_item.clone();
            }
        });
    }
}

pub struct MyApp {
    screen_options: ScreenOptions,
    map_options: MapOptions,
    audio_display: AudioDisplay,
    side_panel: SidePanel,
    paused: bool,
    fps: f32,
    frame_count: i32,
    baseline: Instant,
    trace_on: bool,
    audio_device: AudioQueue<f32>,
    cpu: Cpu,
    texture: egui::TextureHandle,
    tilemap_one_texture: egui::TextureHandle,
    tilemap_two_texture: egui::TextureHandle,
    sprite_texture: egui::TextureHandle,
}

impl MyApp {
    pub fn new(
        frame_count: i32,
        baseline: Instant,
        trace_on: bool,
        audio_device: AudioQueue<f32>,
        cpu: Cpu,
        cc: &eframe::CreationContext<'_>,
    ) -> Self {
        Self {
            screen_options: ScreenOptions::All,
            map_options: MapOptions::Tilemap1,
            audio_display: AudioDisplay::SquareOne,
            side_panel: SidePanel::Cpu,
            paused: false,
            fps: 0.0,
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
            tilemap_one_texture: cc.egui_ctx.load_texture(
                "Noise",
                egui::ColorImage::example(),
                egui::TextureOptions::NEAREST,
            ),
            tilemap_two_texture: cc.egui_ctx.load_texture(
                "Noise",
                egui::ColorImage::example(),
                egui::TextureOptions::NEAREST,
            ),
            sprite_texture: cc.egui_ctx.load_texture(
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

        // PPU Screen Option. Decide which frame to render
        let frame = match self.screen_options {
            ScreenOptions::All => new_frame.unwrap().data,
            ScreenOptions::BackgroundOnly => self.cpu.bus.ppu.bg_screen.to_vec(),
            ScreenOptions::WindowOnly => self.cpu.bus.ppu.win_screen.to_vec(),
            ScreenOptions::SpritesOnly => self.cpu.bus.ppu.spr_screen.to_vec(),
        };

        self.texture.set(
            egui::ColorImage {
                size: [160, 144],
                source_size: egui::Vec2 { x: 160.0, y: 144.0 },
                pixels: frame,
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
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut self.screen_options,
                                ScreenOptions::All,
                                "Normal",
                            );
                            ui.selectable_value(
                                &mut self.screen_options,
                                ScreenOptions::BackgroundOnly,
                                "Background",
                            );
                            ui.selectable_value(
                                &mut self.screen_options,
                                ScreenOptions::WindowOnly,
                                "Window",
                            );
                            ui.selectable_value(
                                &mut self.screen_options,
                                ScreenOptions::SpritesOnly,
                                "Sprites",
                            );
                        });

                        ui.heading("Current PPU State: ");
                        let ppu_str = format!(
                            "Cycles: {}, Scanline: {},\nScroll X, Y: ({}, {}), Window X, Y: ({}, {})\nPPU Status: {:08b}     PPU Control: {:08b}",
                            self.cpu.bus.ppu.cycle,
                            self.cpu.bus.ppu.scanline,
                            self.cpu.bus.ppu.scx,
                            self.cpu.bus.ppu.scy,
                            self.cpu.bus.ppu.wx,
                            self.cpu.bus.ppu.wy,
                            self.cpu.bus.ppu.status.bits(),
                            self.cpu.bus.ppu.control.bits(),
                        );
                        ui.heading(ppu_str);

                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut self.map_options,
                                MapOptions::Tilemap1,
                                "Tile Map 1",
                            );
                            ui.selectable_value(
                                &mut self.map_options,
                                MapOptions::Tilemap2,
                                "Tile Map 2",
                            );
                            ui.selectable_value(
                                &mut self.map_options,
                                MapOptions::Sprites,
                                "Sprites",
                            );
                        });

                        match self.map_options {
                            MapOptions::Tilemap1 => {
                                render::tilemap_one(&mut self.cpu.bus.ppu);

                                self.tilemap_one_texture.set(
                                    egui::ColorImage {
                                        size: [256, 256],
                                        source_size: egui::Vec2 { x: 256.0, y: 256.0 },
                                        pixels: self.cpu.bus.ppu.tilemap_one.to_vec(),
                                    },
                                    egui::TextureOptions::NEAREST,
                                );
                                let tilemap_one = egui::load::SizedTexture::new(
                                    self.tilemap_one_texture.id(),
                                    [256.0, 256.0],
                                );

                                ui.add(
                                    egui::Image::new(tilemap_one)
                                        .fit_to_exact_size(egui::vec2(256.0, 256.0)),
                                );
                            }
                            MapOptions::Tilemap2 => {
                                render::tilemap_two(&mut self.cpu.bus.ppu);

                                self.tilemap_two_texture.set(
                                    egui::ColorImage {
                                        size: [256, 256],
                                        source_size: egui::Vec2 { x: 256.0, y: 256.0 },
                                        pixels: self.cpu.bus.ppu.tilemap_two.to_vec(),
                                    },
                                    egui::TextureOptions::NEAREST,
                                );
                                let tilemap_two = egui::load::SizedTexture::new(
                                    self.tilemap_two_texture.id(),
                                    [256.0, 256.0],
                                );

                                ui.add(
                                    egui::Image::new(tilemap_two)
                                        .fit_to_exact_size(egui::vec2(256.0, 256.0)),
                                );
                            }
                            MapOptions::Sprites => {
                                render::oam_map(&mut self.cpu.bus.ppu);

                                self.sprite_texture.set(
                                    egui::ColorImage {
                                        size: [64, 40],
                                        source_size: egui::Vec2 { x: 64.0, y: 40.0 },
                                        pixels: self.cpu.bus.ppu.sprites.to_vec(),
                                    },
                                    egui::TextureOptions::NEAREST,
                                );
                                let sprites = egui::load::SizedTexture::new(
                                    self.sprite_texture.id(),
                                    [64.0, 40.0],
                                );
                                ui.add(
                                    egui::Image::new(sprites)
                                        .fit_to_exact_size(egui::vec2(3.0 * 64.0, 3.0 * 40.0)),
                                );
                            }
                        }
                    }
                    SidePanel::Apu => {
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut self.audio_display,
                                AudioDisplay::SquareOne,
                                "Square 1",
                            );
                            ui.selectable_value(
                                &mut self.audio_display,
                                AudioDisplay::SquareTwo,
                                "Square 2",
                            );
                            ui.selectable_value(
                                &mut self.audio_display,
                                AudioDisplay::Wave,
                                "Wave",
                            );
                            ui.selectable_value(
                                &mut self.audio_display,
                                AudioDisplay::Noise,
                                "Noise",
                            );
                        });

                        let points = match self.audio_display {
                            AudioDisplay::SquareOne => {
                                let points: PlotPoints = self.cpu.bus.apu.square1_output.iter().enumerate().map(|(index, value)| {
                                    [index as f64, *value as f64]
                                }).collect();
                                points
                            }
                            AudioDisplay::SquareTwo => {
                                let points: PlotPoints = self.cpu.bus.apu.square2_output.iter().enumerate().map(|(index, value)| {
                                    [index as f64, *value as f64]
                                }).collect();
                                points
                            }
                            AudioDisplay::Wave => {
                                let points: PlotPoints = self.cpu.bus.apu.wave_output.iter().enumerate().map(|(index, value)| {
                                    [index as f64, *value as f64]
                                }).collect();
                                points
                            }
                            AudioDisplay::Noise => {
                                let points: PlotPoints = self.cpu.bus.apu.noise_output.iter().enumerate().map(|(index, value)| {
                                    [index as f64, *value as f64]
                                }).collect();
                                points
                            }
                        };

                        let line = Line::new("S1", points);
                        Plot::new("my_plot").view_aspect(2.0).show(ui, |plot_ui| plot_ui.line(line));

                        ui.heading("Play only these audios:");

                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut self.cpu.bus.apu.audio_select,
                                apu::AudioSelect::All,
                                "All",
                            );
                            ui.selectable_value(
                                &mut self.cpu.bus.apu.audio_select,
                                apu::AudioSelect::SquareOne,
                                "Square 1",
                            );
                            ui.selectable_value(
                                &mut self.cpu.bus.apu.audio_select,
                                apu::AudioSelect::SquareTwo,
                                "Square 2",
                            );
                            ui.selectable_value(
                                &mut self.cpu.bus.apu.audio_select,
                                apu::AudioSelect::Wave,
                                "Wave",
                            );
                            ui.selectable_value(
                                &mut self.cpu.bus.apu.audio_select,
                                apu::AudioSelect::Noise,
                                "Noise",
                            );
                        });
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

            ui.heading(cpu_state);
            ui.heading(format!("FPS: {}", self.fps));
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
        if self.frame_count == 0 {
            self.baseline = Instant::now();
        } else if self.frame_count == 30 {
            let thirty_frame_time = self.baseline.elapsed().as_secs_f32();
            self.frame_count = 1;
            self.baseline = Instant::now();
            let fps = 30.0 / thirty_frame_time;
            //println!("FPS is {fps}");
            self.fps = fps;
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
            while self.audio_device.size() > 4500 {

            }

            // check user input
            //sdl2_setup::get_user_input(&mut self.event_pump, &mut self.cpu.bus.joypad);

            // If FPS enabled, increment counter
            self.frame_count += 1;

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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ScreenOptions {
    All,
    SpritesOnly,
    BackgroundOnly,
    WindowOnly,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum MapOptions {
    Tilemap1,
    Tilemap2,
    Sprites,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AudioDisplay {
    SquareOne,
    SquareTwo,
    Wave,
    Noise,
}
