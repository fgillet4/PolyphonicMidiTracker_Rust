use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use std::sync::Arc;

#[derive(Default)]
pub struct EditorState {
    pub fft_visible: bool,
}

pub fn create_editor(params: Arc<dyn Params>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params,
        (),
        |_, _| {},
        |ctx, setter, state| {
            let params = state.params.lock();
            let editor_state = params.editor_state.read();
            
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.heading("Guitar MIDI Tracker");
                
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                
                // Main control section
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.heading("Input");
                        ui.add(egui::Slider::new(
                            &mut setter.setter(&params.input_gain), 
                            -12.0..=12.0
                        ).text("Gain").suffix(" dB"));
                    });
                    
                    ui.add_space(20.0);
                    
                    ui.vertical(|ui| {
                        ui.heading("Detection");
                        ui.add(egui::Slider::new(
                            &mut setter.setter(&params.sensitivity), 
                            0.1..=1.0
                        ).text("Sensitivity"));
                        ui.add(egui::Slider::new(
                            &mut setter.setter(&params.max_polyphony), 
                            1..=12
                        ).text("Max Polyphony"));
                    });
                });
                
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                
                // Learning section
                ui.collapsing("Learning Mode", |ui| {
                    ui.checkbox(&mut setter.setter(&params.learning_mode), "Enable Learning Mode");
                    
                    if params.learning_mode.value() {
                        ui.add_space(10.0);
                        ui.add(egui::Slider::new(
                            &mut setter.setter(&params.learning_note), 
                            40.0..=90.0
                        ).text("Learning Note"));
                        
                        ui.horizontal(|ui| {
                            if ui.button("Save Learned Data").clicked() {
                                setter.set_parameter(&params.save_learned_data, true);
                            }
                            
                            if ui.button("Load Learned Data").clicked() {
                                setter.set_parameter(&params.load_learned_data, true);
                            }
                        });
                    }
                });
                
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
                
                // Visualization section - placeholder
                ui.collapsing("Visualization", |ui| {
                    ui.checkbox(&mut setter.setter(&params.editor_state.as_ref().fft_visible), "Show FFT");
                    
                    if editor_state.fft_visible {
                        ui.add_space(10.0);
                        
                        // Placeholder for FFT visualization
                        let plot_size = egui::Vec2::new(ui.available_width(), 200.0);
                        ui.label("FFT Visualization - Not Implemented");
                        ui.add(egui::widgets::Frame::canvas(&ctx.style()).fill(egui::Color32::BLACK)
                            .show(ui, |ui| {
                                ui.allocate_space(plot_size);
                            })
                        );
                    }
                });
            });
        },
    )
}
