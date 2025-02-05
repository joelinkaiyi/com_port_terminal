use eframe::egui;
use serialport::{available_ports, SerialPort, SerialPortType};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;
use std::thread;

struct ComPortApp {
    available_ports: Vec<String>,
    selected_port: Option<String>,
    baud_rates: Vec<u32>,
    selected_baud_rate: u32,
    input_buffer: String,
    output_buffer: String,
    port_handle: Option<Box<dyn SerialPort>>,
    rx: Receiver<String>,
    tx: Sender<String>,
}

impl Default for ComPortApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        
        Self {
            available_ports: Vec::new(),
            selected_port: None,
            baud_rates: vec![9600, 19200, 38400, 57600, 115200],
            selected_baud_rate: 9600,
            input_buffer: String::new(),
            output_buffer: String::new(),
            port_handle: None,
            rx,
            tx,
        }
    }
}

impl ComPortApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        app.refresh_ports();
        app
    }

    fn refresh_ports(&mut self) {
        self.available_ports.clear();
        if let Ok(ports) = available_ports() {
            for p in ports {
                self.available_ports.push(p.port_name);
            }
        }
    }

    fn connect_port(&mut self) {
        if let Some(port_name) = &self.selected_port {
            match serialport::new(port_name, self.selected_baud_rate)
                .timeout(Duration::from_millis(10))
                .open()
            {
                Ok(port) => {
                    self.port_handle = Some(port);
                    let tx = self.tx.clone();
                    let mut port = self.port_handle.as_mut().unwrap().try_clone().unwrap();
                    
                    thread::spawn(move || {
                        let mut serial_buf: Vec<u8> = vec![0; 1000];
                        loop {
                            if let Ok(t) = port.read(serial_buf.as_mut_slice()) {
                                if t > 0 {
                                    if let Ok(s) = String::from_utf8(serial_buf[..t].to_vec()) {
                                        tx.send(s).unwrap();
                                    }
                                }
                            }
                            thread::sleep(Duration::from_millis(10));
                        }
                    });
                }
                Err(e) => {
                    println!("Error opening port: {}", e);
                }
            }
        }
    }

    fn disconnect_port(&mut self) {
        self.port_handle = None;
    }

    fn send_data(&mut self) {
        if let Some(port) = &mut self.port_handle {
            if let Ok(_) = port.write(self.input_buffer.as_bytes()) {
                self.input_buffer.clear();
            }
        }
    }
}

impl eframe::App for ComPortApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 接收port數據
        while let Ok(data) = self.rx.try_recv() {
            self.output_buffer.push_str(&data);
            if self.output_buffer.len() > 1000 {
                self.output_buffer = self.output_buffer.split_off(self.output_buffer.len() - 1000);
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("refresh ports").clicked() {
                    self.refresh_ports();
                }

                egui::ComboBox::from_label("select port")
                    .selected_text(self.selected_port.as_deref().unwrap_or(""))
                    .show_ui(ui, |ui| {
                        for port in &self.available_ports {
                            ui.selectable_value(&mut self.selected_port, Some(port.clone()), port);
                        }
                    });

                egui::ComboBox::from_label("baud rate")
                    .selected_text(self.selected_baud_rate.to_string())
                    .show_ui(ui, |ui| {
                        for &rate in &self.baud_rates {
                            ui.selectable_value(&mut self.selected_baud_rate, rate, rate.to_string());
                        }
                    });

                if self.port_handle.is_none() {
                    if ui.button("connect").clicked() {
                        self.connect_port();
                    }
                } else {
                    if ui.button("disconnect").clicked() {
                        self.disconnect_port();
                    }
                }
            });

            ui.separator();

            // 輸出顯示區域
            ui.group(|ui| {
                ui.label("output area");
                ui.add_sized(
                    [ui.available_width(), 200.0],
                    egui::TextEdit::multiline(&mut self.output_buffer)
                        .desired_rows(10)
                        .lock_focus(true),
                );
            });

            ui.separator();

            // 輸入區域
            ui.horizontal(|ui| {
                let text_edit = ui.add_sized(
                    [ui.available_width() - 60.0, 30.0],
                    egui::TextEdit::singleline(&mut self.input_buffer),
                );

                if ui.button("send").clicked() || text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.send_data();
                }
            });
        });

        // 請求持續更新以接收串口數據
        ctx.request_repaint();
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    
    eframe::run_native(
        "COM Port Terminal",
        options,
        Box::new(|cc| Box::new(ComPortApp::new(cc)))
    )
} 