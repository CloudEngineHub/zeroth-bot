use anyhow::Result;
use runtime::hal::{Servo, MAX_SERVOS, TorqueMode, ServoRegister};
use cursive::views::{TextView, LinearLayout, DummyView, Panel, Dialog, EditView, SelectView};
use cursive::traits::*;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use std::sync::atomic::{AtomicI16, Ordering, AtomicBool};
use cursive::theme::{Color, ColorStyle, BaseColor};
use cursive::view::Nameable;
use std::sync::OnceLock;
use std::fs::File;
use std::io::Write;
use chrono::Local;
use serde_json::{json, Value};

static CALIBRATION_POSITION: AtomicI16 = AtomicI16::new(-1);
static CURRENT_POSITION: AtomicI16 = AtomicI16::new(0);
static CAPTURE_IN_PROGRESS: AtomicBool = AtomicBool::new(false);
static CAPTURE_STATE: OnceLock<Arc<Mutex<CaptureState>>> = OnceLock::new();

// Add this near the top of your file
static UNRESPONSIVE_SERVOS: OnceLock<Arc<Mutex<Vec<bool>>>> = OnceLock::new();

// At the top of the file, add this array of joint names
const JOINT_NAMES: [&str; 16] = [
    "R Ank", "R Knee", "R Hip R", "R Hip Y", "R Hip P",
    "L Ank", "L Knee", "L Hip R", "L Hip Y", "L Hip P",
    "R Elb", "R Sh Y", "R Sh P",
    "L Elb", "L Sh Y", "L Sh P"
];

struct CaptureState {
    file: Option<File>,
    captures: Vec<Value>,
    name: String,
}

fn show_hints(s: &mut cursive::Cursive) {
    let hints = vec![
        "Up/Down - Select servo",
        "Enter - Open servo settings",
        "T - Toggle torque",
        "[ - Start calibration",
        "] - End calibration",
        "c - Capture current position",
        "C - End and save capture",
        "Q - Quit",
        "H - Show this help",
    ];

    let content = hints.join("\n");

    s.add_layer(
        Dialog::around(TextView::new(content))
            .title("Hints")
            .button("Close", |s| { s.pop_layer(); })
    );
}

fn main() -> Result<()> {
    let servo = Arc::new(Servo::new()?);

    // Enable continuous readout
    servo.enable_readout()?;

    let mut siv = cursive::default();

    // Create a layout for our servo data
    let mut layout = LinearLayout::vertical();

    // Add header
    let header = LinearLayout::horizontal()
        .child(TextView::new("ID").center().fixed_width(4))
        .child(TextView::new("Joint").center().fixed_width(7))
        .child(TextView::new("Pos").center().fixed_width(8))
        .child(TextView::new("Spd").center().fixed_width(8))
        .child(TextView::new("Load").center().fixed_width(8))
        .child(TextView::new("Torque").center().fixed_width(8))
        .child(TextView::new("Volt").center().fixed_width(6))
        .child(TextView::new("Temp").center().fixed_width(6))
        .child(TextView::new("Curr").center().fixed_width(6))
        .child(TextView::new("Status").center().fixed_width(8))
        .child(TextView::new("Torq Lim").center().fixed_width(8));
    layout.add_child(header);

    // Add rows for each servo
    for i in 0..MAX_SERVOS {
        let joint_name = JOINT_NAMES.get(i).unwrap_or(&"Unknown");
        let row = LinearLayout::horizontal()
            .child(TextView::new(format!("{:2}", i + 1)).center().with_name(format!("ID {}", i)).fixed_width(4))
            .child(TextView::new(*joint_name).center().with_name(format!("Joint {}", i)).fixed_width(7))
            .child(TextView::new("----").center().with_name(format!("CurrPos {}", i)).fixed_width(8))
            .child(TextView::new("----").center().with_name(format!("CurrSpd {}", i)).fixed_width(8))
            .child(TextView::new("----").center().with_name(format!("Load {}", i)).fixed_width(8))
            .child(TextView::new("----").center().with_name(format!("Torque {}", i)).fixed_width(8))
            .child(TextView::new("----").center().with_name(format!("Volt {}", i)).fixed_width(6))
            .child(TextView::new("----").center().with_name(format!("Temp {}", i)).fixed_width(6))
            .child(TextView::new("----").center().with_name(format!("Curr {}", i)).fixed_width(6))
            .child(TextView::new("----").center().with_name(format!("Status {}", i)).fixed_width(8))
        .child(TextView::new("----").center().with_name(format!("TorqLim {}", i)).fixed_width(8));

            // .child(TextView::new("----").center().with_name(format!("Async {}", i)).fixed_width(6))
            // .child(TextView::new("----").center().with_name(format!("Lock {}", i)).fixed_width(6));
        layout.add_child(row.with_name(format!("servo_row_{}", i)));
    }

    // Add a dummy view to push the task count to the bottom
    layout.add_child(DummyView.full_height());

    // Add task run count at the bottom
    layout.add_child(
        Panel::new(
            LinearLayout::horizontal()
                .child(DummyView)
                .child(
                    LinearLayout::vertical()
                        .child(TextView::new("Task Run Count: 0").with_name("Task Count"))
                        .child(TextView::new("Last Update: N/A").with_name("Last Update"))
                        .child(TextView::new("Servo polling rate: N/A").with_name("Servo polling rate"))
                )
                .child(DummyView.fixed_width(2))
                .child(
                    LinearLayout::vertical()
                        .child(TextView::new("Min Angle: ----").with_name("MinAngle"))
                        .child(TextView::new("Max Angle: ----").with_name("MaxAngle"))
                        .child(TextView::new("Offset: ----").with_name("Offset"))
                )
                .child(DummyView.fixed_width(2))
                .child(
                    LinearLayout::vertical()
                        .child(TextView::new("Angle Range: ----°").with_name("AngleRange"))
                )
        )
        .title("Statistics")
        .full_width()
    );

    // Add instructions
    layout.add_child(
        TextView::new("Press 'H' for help")
            .center()
            .full_width()
    );

    layout.add_child(TextView::new("Calibration Pos: ----").with_name("CalibrationPos"));

    siv.add_fullscreen_layer(layout);

    // Set up a timer to update the UI
    siv.set_fps(30);

    // Clone Arc for the callback
    let servo_clone = Arc::clone(&servo);

    // Add a variable to keep track of the selected servo
    let selected_servo = Arc::new(Mutex::new(0));

    // Add variables for last update time and task count
    let last_update_time = Arc::new(Mutex::new(Instant::now()));

    siv.add_global_callback('q', |s| s.quit());

    // Modify Up and Down callbacks to wrap around
    let servo_clone_up = Arc::clone(&servo);
    let selected_servo_up = Arc::clone(&selected_servo);
    siv.add_global_callback(cursive::event::Event::Key(cursive::event::Key::Up), move |s| {
        let mut selected = selected_servo_up.lock().unwrap();
        *selected = (*selected + MAX_SERVOS - 1) % MAX_SERVOS;
        update_selected_row(s, *selected);
        update_angle_limits(s, *selected as u8 + 1, &servo_clone_up);
    });

    let servo_clone_down = Arc::clone(&servo);
    let selected_servo_down = Arc::clone(&selected_servo);
    siv.add_global_callback(cursive::event::Event::Key(cursive::event::Key::Down), move |s| {
        let mut selected = selected_servo_down.lock().unwrap();
        *selected = (*selected + 1) % MAX_SERVOS;
        update_selected_row(s, *selected);
        update_angle_limits(s, *selected as u8 + 1, &servo_clone_down);
    });

    siv.add_global_callback('h', show_hints);

    let servo_clone_enter = Arc::clone(&servo);
    let selected_servo_enter = Arc::clone(&selected_servo);
    siv.add_global_callback(cursive::event::Event::Key(cursive::event::Key::Enter), move |s| {
        // Check if a settings dialog is already open
        if s.find_name::<Dialog>("servo_settings").is_some() || s.find_name::<Dialog>("capture_dialog").is_some(){
            return; // Do nothing if a dialog is already open
        }

        let selected = *selected_servo_enter.lock().unwrap();
        let servo_id = selected as u8 + 1;
        open_servo_settings(s, servo_id, Arc::clone(&servo_clone_enter));
    });

    let servo_clone_toggle = Arc::clone(&servo);
    let selected_servo_toggle = Arc::clone(&selected_servo);
    siv.add_global_callback('t', move |s| {
        let selected = *selected_servo_toggle.lock().unwrap();
        let servo_id = selected as u8 + 1;
        toggle_servo_torque(s, servo_id, Arc::clone(&servo_clone_toggle));
    });

    let servo_clone_calibrate_start = Arc::clone(&servo);
    let selected_servo_calibrate_start = Arc::clone(&selected_servo);
    siv.add_global_callback('[', move |s| {
        let selected = *selected_servo_calibrate_start.lock().unwrap();
        let servo_id = selected as u8 + 1;
        start_calibration(s, servo_id, Arc::clone(&servo_clone_calibrate_start));
    });

    let servo_clone_calibrate_end = Arc::clone(&servo);
    let selected_servo_calibrate_end = Arc::clone(&selected_servo);
    siv.add_global_callback(']', move |s| {
        let selected = *selected_servo_calibrate_end.lock().unwrap();
        let servo_id = selected as u8 + 1;
        end_calibration(s, servo_id, Arc::clone(&servo_clone_calibrate_end));
    });

    let mut update_count = 0;
    let servo_clone_for_scan = Arc::clone(&servo);

    siv.set_global_callback(cursive::event::Event::Refresh, move |s| {
        update_count += 1;

        match servo_clone.read_continuous() {
            Ok(data) => {
                for (i, servo_info) in data.servo.iter().enumerate() {
                    s.call_on_name(&format!("CurrPos {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:4}", servo_info.current_location));
                    });
                    s.call_on_name(&format!("CurrSpd {}", i), |view: &mut TextView| {
                        let speed = servo_info.current_speed as u16 & 0x7FFF; // Remove 15th bit
                        let sign = if servo_info.current_speed as u16 & 0x8000 != 0 { '-' } else { '+' };
                        view.set_content(format!("{}{:4}", sign, speed));
                    });
                    s.call_on_name(&format!("Load {}", i), |view: &mut TextView| {
                        let speed = servo_info.current_load as u16 & 0x3FF; // Remove 10th bit
                        let sign = if servo_info.current_load as u16 & 0x400 != 0 { '-' } else { '+' };
                        view.set_content(format!("{}{:4}", sign, speed));
                    });
                    update_torque_display(s, (i + 1) as u8, servo_info.torque_switch);
                    s.call_on_name(&format!("TorqLim {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:4}", servo_info.torque_limit));
                    });
                    s.call_on_name(&format!("Volt {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:2.1}V", servo_info.current_voltage as f32 / 10.0));
                    });
                    s.call_on_name(&format!("Temp {}", i), |view: &mut TextView| {
                        view.set_content(format!("{}°C", servo_info.current_temperature));
                    });
                    s.call_on_name(&format!("Curr {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:4}", servo_info.current_current));
                    });
                    s.call_on_name(&format!("Status {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:05b}", servo_info.servo_status));
                    });
                    if servo_info.servo_status != 0 {
                        s.call_on_name(&format!("Status {}", i), |view: &mut TextView| {
                            view.set_style(ColorStyle::highlight());
                        });
                    } else {
                        s.call_on_name(&format!("Status {}", i), |view: &mut TextView| {
                            view.set_style(ColorStyle::default());
                        });
                    }
                    s.call_on_name(&format!("Lock {}", i), |view: &mut TextView| {
                        view.set_content(format!("{:4}", servo_info.lock_mark));
                    });

                    // Check servo responsiveness every 10th update
                    if update_count % 50 == 0 {
                        let servo_id = i as u8 + 1;
                        let mut is_responsive = match servo_clone_for_scan.scan(servo_id) {
                            Ok(true) => true,
                            Ok(false) => false,
                            Err(_) => false,
                        };
                        
                        let mut unresponsive_servos = UNRESPONSIVE_SERVOS.get().unwrap().lock().unwrap();
                        unresponsive_servos[i] = !is_responsive;
                        
                        let style = if is_responsive {
                            ColorStyle::default()
                        } else {
                            ColorStyle::highlight()
                        };
                        
                        // Apply style to ID and Joint name
                        s.call_on_name(&format!("ID {}", i), |view: &mut TextView| {
                            view.set_style(style);
                        });
                        s.call_on_name(&format!("Joint {}", i), |view: &mut TextView| {
                            view.set_style(style);
                        });

                        let selected = *selected_servo.lock().unwrap();

                        if selected == i && unresponsive_servos[i] {
                            s.call_on_name(&format!("ID {}", i), |view: &mut TextView| {
                                view.set_style(ColorStyle::secondary());
                            });
                        } else if selected == i {
                            s.call_on_name(&format!("ID {}", i), |view: &mut TextView| {
                                view.set_style(ColorStyle::secondary());
                            });
                            s.call_on_name(&format!("Joint {}", i), |view: &mut TextView| {
                                view.set_style(ColorStyle::secondary());
                            });
                        }
                    }
                }
                let mut last_update = last_update_time.lock().unwrap();
                let now = Instant::now();
                let time_delta = now.duration_since(*last_update);

                let update_rate = if data.task_run_count > 0 {
                    1000.0 / (time_delta.as_millis() as f64 / data.task_run_count as f64)
                } else {
                    0.0
                };

                s.call_on_name("Task Count", |view: &mut TextView| {
                    view.set_content(format!("Task Run Count: {}", data.task_run_count));
                });
                s.call_on_name("Last Update", |view: &mut TextView| {
                    view.set_content(format!("Last Update: {:?} ago", time_delta));
                });
                s.call_on_name("Servo polling rate", |view: &mut TextView| {
                    view.set_content(format!("Servo polling rate: {:.2} Hz", update_rate));
                });

                *last_update = now;

                let selected = *selected_servo.lock().unwrap();
                let current_pos = data.servo[selected].current_location;
                CURRENT_POSITION.store(current_pos, Ordering::Relaxed);

                s.call_on_name("CalibrationPos", |view: &mut TextView| {
                    let cal_pos = CALIBRATION_POSITION.load(Ordering::Relaxed);
                    view.set_content(format!("Calibration Pos: {}", if cal_pos >= 0 { cal_pos.to_string() } else { "----".to_string() }));
                });
            }
            Err(e) => {
                s.add_layer(Dialog::info(format!("Error reading servo data: {}", e)));
            }
        }

        // Reset update count to avoid potential overflow
        if update_count >= 1000 {
            update_count = 0;
        }
    });

    // Initialize the OnceLock at the start of main
    UNRESPONSIVE_SERVOS.get_or_init(|| Arc::new(Mutex::new(vec![false; MAX_SERVOS])));

    // Initialize capture state
    CAPTURE_STATE.get_or_init(|| Arc::new(Mutex::new(CaptureState {
        file: None,
        captures: Vec::new(),
        name: String::new(),
    })));

    siv.set_user_data(servo.clone());

    // Add capture callbacks
    siv.add_global_callback('c', |s| handle_capture(s, false));
    siv.add_global_callback('C', |s| handle_capture(s, true));

    siv.run();

    Ok(())
}

// Update the update_selected_row function
fn update_selected_row(s: &mut cursive::Cursive, selected: usize) {
    let unresponsive_servos = UNRESPONSIVE_SERVOS.get().unwrap().lock().unwrap();
    
    for i in 0..MAX_SERVOS {
        let style = if unresponsive_servos[i] {
            ColorStyle::highlight()
        } else if i == selected {
            ColorStyle::secondary()
        } else {
            ColorStyle::default()
        };

        s.call_on_name(&format!("ID {}", i), |view: &mut TextView| {
            view.set_style(style);
        });
        s.call_on_name(&format!("Joint {}", i), |view: &mut TextView| {
            view.set_style(style);
        });
    }
}

fn update_angle_limits(s: &mut cursive::Cursive, servo_id: u8, servo: &Arc<Servo>) {
    match servo.read_angle_limits(servo_id) {
        Ok((min_angle, max_angle)) => {
            s.call_on_name("MinAngle", |view: &mut TextView| {
                view.set_content(format!("Min Angle: {}", min_angle));
            });
            s.call_on_name("MaxAngle", |view: &mut TextView| {
                view.set_content(format!("Max Angle: {}", max_angle));
            });
            
            // Calculate and display the angle range
            let angle_range = (max_angle as f64 - min_angle as f64) / 4096.0 * 360.0;
            s.call_on_name("AngleRange", |view: &mut TextView| {
                view.set_content(format!("Angle Range: {:.2}°", angle_range));
            });
        }
        Err(e) => {
            s.call_on_name("MinAngle", |view: &mut TextView| {
                view.set_content("Min Angle: Error".to_string());
            });
            s.call_on_name("MaxAngle", |view: &mut TextView| {
                view.set_content("Max Angle: Error".to_string());
            });
            s.call_on_name("AngleRange", |view: &mut TextView| {
                view.set_content("Angle Range: Error".to_string());
            });
            eprintln!("Error reading angle limits: {}", e);
        }
    }

    // Update offset
    match servo.read(servo_id, ServoRegister::PositionCorrection, 2) {
        Ok(data) => {
            let offset = i16::from_le_bytes([data[0], data[1]]);
            s.call_on_name("Offset", |view: &mut TextView| {
                view.set_content(format!("Offset: {}", offset));
            });
        }
        Err(e) => {
            s.call_on_name("Offset", |view: &mut TextView| {
                view.set_content("Offset: Error".to_string());
            });
            eprintln!("Error reading offset: {}", e);
        }
    }
}

fn open_servo_settings(s: &mut cursive::Cursive, servo_id: u8, servo: Arc<Servo>) {
    // Read the current torque mode and torque limit
    let (current_torque_mode, current_torque_limit) = match servo.read_info(servo_id) {
        Ok(info) => {
            let mode = if info.torque_switch == 0 {
                TorqueMode::Disabled
            } else {
                TorqueMode::Enabled
            };
            (mode, info.torque_limit)
        },
        Err(_) => (TorqueMode::Enabled, 1000), // Default values if we can't read the current state
    };

    let dialog = Dialog::new()
        .title(format!("Servo {} Settings", servo_id))
        .content(
            LinearLayout::vertical()
                .child(TextView::new("Position:"))
                .child(EditView::new().with_name("position"))
                .child(TextView::new("Speed:"))
                .child(EditView::new().with_name("speed"))
                .child(TextView::new("Torque:"))
                .child(SelectView::new()
                    .item("Enabled", Arc::new(TorqueMode::Enabled))
                    .item("Disabled", Arc::new(TorqueMode::Disabled))
                    .selected(match current_torque_mode {
                        TorqueMode::Enabled => 0,
                        TorqueMode::Disabled => 1,
                        _ => 0, // Default to Enabled for other cases
                    })
                    .popup()
                    .with_name("torque"))
                .child(TextView::new("Torque Limit:"))
                .child(EditView::new()
                    .content(current_torque_limit.to_string())
                    .with_name("torque_limit"))
                .child(TextView::new("Offset:"))
                .child(EditView::new().with_name("offset"))
        )
        .button("Apply", move |s| {
            let position = s.call_on_name("position", |view: &mut EditView| {
                view.get_content().parse::<i16>().ok()
            }).unwrap();
            let speed = s.call_on_name("speed", |view: &mut EditView| {
                view.get_content().parse::<u16>().unwrap_or(0)
            }).unwrap();
            let torque_mode = s.call_on_name("torque", |view: &mut SelectView<Arc<TorqueMode>>| {
                view.selection().unwrap_or_else(|| Arc::new(TorqueMode::Enabled.into()))
            }).unwrap();
            let offset = s.call_on_name("offset", |view: &mut EditView| {
                view.get_content().parse::<i16>().ok()
            }).unwrap();
            let torque_limit = s.call_on_name("torque_limit", |view: &mut EditView| {
                view.get_content().parse::<u16>().unwrap_or(0)
            }).unwrap();

            // Apply settings
            if let Err(e) = servo.set_torque_mode(servo_id, (**torque_mode).clone()) {
                s.add_layer(Dialog::info(format!("Error setting torque mode: {}", e)));
            }

            // Move servo only if position is provided
            if let Some(pos) = position {
                if let Err(e) = servo.move_servo(servo_id, pos, 0, speed) {
                    s.add_layer(Dialog::info(format!("Error moving servo: {}", e)));
                }
            }

            // Set offset if provided
            if let Some(off) = offset {
                if let Err(e) = set_servo_offset(servo_id, off, &servo) {
                    s.add_layer(Dialog::info(format!("Error setting offset: {}", e)));
                } else {
                    s.call_on_name("Offset", |view: &mut TextView| {
                        view.set_content(format!("Offset: {}", off));
                    });
                }
            }

            // Set torque limit if provided
            if torque_limit > 0 {
                if let Err(e) = set_servo_torque(servo_id, torque_limit, &servo) {
                    s.add_layer(Dialog::info(format!("Error setting torque limit: {}", e)));
                } else {
                    s.call_on_name("CurrentTorque", |view: &mut TextView| {
                        view.set_content(format!("Current Torque: {}", torque_limit));
                    });
                }
            }

            s.pop_layer();
        })
        .button("Cancel", |s| {
            s.pop_layer();
        })
        .with_name("servo_settings"); // Add this line to name the dialog

    s.add_layer(dialog);
}

fn set_servo_offset(servo_id: u8, offset: i16, servo: &Arc<Servo>) -> Result<()> {
    let offset_value = if offset < 0 {
        (offset.abs() as u16) | 0x800 // Set bit 11 for negative values
    } else {
        offset as u16
    };

    // Unlock EEPROM
    servo.write(servo_id, ServoRegister::LockMark, &[0])?;
    std::thread::sleep(Duration::from_millis(10));

    // Write new offset
    servo.write_servo_memory(servo_id, ServoRegister::PositionCorrection, offset_value)?;
    std::thread::sleep(Duration::from_millis(10));

    // Lock EEPROM
    servo.write(servo_id, ServoRegister::LockMark, &[1])?;

    Ok(())
}

fn toggle_servo_torque(s: &mut cursive::Cursive, servo_id: u8, servo: Arc<Servo>) {
    let servo_clone = Arc::clone(&servo);
    
    match servo_clone.read_info(servo_id) {
        Ok(info) => {
            let new_torque_mode = if info.torque_switch == 0 {
                TorqueMode::Enabled
            } else {
                TorqueMode::Disabled
            };
            
            if let Err(e) = servo_clone.set_torque_mode(servo_id, new_torque_mode.clone()) {
                s.add_layer(Dialog::info(format!("Error setting torque mode: {}", e)));
            } else {
                // Update the UI immediately
                let new_torque_value = match new_torque_mode {
                    TorqueMode::Enabled => 1,
                    TorqueMode::Disabled => 0,
                    TorqueMode::Stiff => 1,
                };
                update_torque_display(s, servo_id, new_torque_value);
            }
        }
        Err(e) => {
            s.add_layer(Dialog::info(format!("Error reading servo info: {}", e)));
        }
    }
}

fn update_torque_display(s: &mut cursive::Cursive, servo_id: u8, torque_value: u8) {
    s.call_on_name(&format!("Torque {}", servo_id as usize - 1), |view: &mut TextView| {
        view.set_content(format!("{:4}", torque_value));
        if torque_value == 1 {
            view.set_style(ColorStyle::secondary());
        } else {
            view.set_style(ColorStyle::default());
        }
    });
}

fn start_calibration(s: &mut cursive::Cursive, servo_id: u8, servo: Arc<Servo>) {
    if let Err(e) = servo.write(servo_id, ServoRegister::PositionCorrection, &[0, 0]) {
        s.add_layer(Dialog::info(format!("Error setting position correction to 0: {}", e)));
        return;
    }

    std::thread::sleep(Duration::from_millis(20));

    match servo.read_info(servo_id) {
        Ok(info) => {
            CALIBRATION_POSITION.store(info.current_location, Ordering::Relaxed);
            s.add_layer(Dialog::info(format!("Calibration started for servo {}. Current position: {}", servo_id, info.current_location)));
        }
        Err(e) => {
            s.add_layer(Dialog::info(format!("Error reading servo info: {}", e)));
        }
    }
}

fn end_calibration(s: &mut cursive::Cursive, servo_id: u8, servo: Arc<Servo>) {
    let min_pos = CALIBRATION_POSITION.load(Ordering::Relaxed);
    if min_pos < 0 {
        s.add_layer(Dialog::info("Please start calibration first by pressing '['"));
        return;
    }

    let mut max_pos = CURRENT_POSITION.load(Ordering::Relaxed);

    if max_pos < min_pos {
        max_pos += 4096;
    }

    let offset_value = min_pos + (max_pos - min_pos) / 2 - 2048;

    // Convert offset to 12-bit signed value
    let offset_value = if offset_value < 0 {
        offset_value.abs() as u16 | 0x800 // (set negative)
    } else {
        if offset_value > 2048 {
            (offset_value - 4096).abs() as u16 | 0x800
        } else {
            offset_value as u16
        }
    };

    // Calculate new limits
    let min_angle = 2048 - (max_pos - min_pos) / 2;
    let max_angle = 2048 + (max_pos - min_pos) / 2;

    // Write new values to EEPROM
    if let Err(e) = write_calibration_to_eeprom(servo_id, &servo, offset_value, min_angle, max_angle) {
        s.add_layer(Dialog::info(format!("Error writing calibration to EEPROM: {}", e)));
        return;
    }

    // Update the UI
    s.call_on_name("MinAngle", |view: &mut TextView| {
        view.set_content(format!("Min Angle: {}", min_angle));
    });
    s.call_on_name("MaxAngle", |view: &mut TextView| {
        view.set_content(format!("Max Angle: {}", max_angle));
    });
    s.call_on_name("Offset", |view: &mut TextView| {
        view.set_content(format!("Offset: {}", offset_value));
    });

    CALIBRATION_POSITION.store(-1, Ordering::Relaxed);
    s.add_layer(Dialog::info(format!("Calibration completed for servo {}. New offset: {}", servo_id, offset_value)));
}

fn write_calibration_to_eeprom(servo_id: u8, servo: &Servo, offset: u16, min_angle: i16, max_angle: i16) -> Result<()> {
    // Unlock EEPROM
    servo.write(servo_id, ServoRegister::LockMark, &[0])?;
    std::thread::sleep(Duration::from_millis(20));

    // Write new offset
    servo.write_servo_memory(servo_id, ServoRegister::PositionCorrection, offset)?;
    std::thread::sleep(Duration::from_millis(20));

    // Write new limits
    for try_num in 0..3 {
        servo.write_servo_memory(servo_id, ServoRegister::MinAngleLimit, min_angle as u16)?;
        std::thread::sleep(Duration::from_millis(20));
        let read_min = servo.read(servo_id, ServoRegister::MinAngleLimit, 2)?;
        let read_min = u16::from_le_bytes([read_min[0], read_min[1]]);
        if read_min == min_angle as u16 {
            break;
        }
        if try_num == 2 {
            return Err(anyhow::anyhow!("Failed to write MinAngleLimit after 3 attempts"));
        }
    }

    for try_num in 0..3 {
        servo.write_servo_memory(servo_id, ServoRegister::MaxAngleLimit, max_angle as u16)?;
        std::thread::sleep(Duration::from_millis(20));
        let read_max = servo.read(servo_id, ServoRegister::MaxAngleLimit, 2)?;
        let read_max = u16::from_le_bytes([read_max[0], read_max[1]]);
        if read_max == max_angle as u16 {
            break;
        }
        if try_num == 2 {
            return Err(anyhow::anyhow!("Failed to write MaxAngleLimit after 3 attempts"));
        }
    }

    // Lock EEPROM
    servo.write(servo_id, ServoRegister::LockMark, &[1])?;

    Ok(())
}

fn handle_capture(s: &mut cursive::Cursive, end: bool) {
    if !CAPTURE_IN_PROGRESS.load(Ordering::Relaxed) && !end {
        // Prompt for capture name
        s.add_layer(
            Dialog::new()
                .title("New Capture")
                .content(EditView::new().with_name("capture_name"))
                .button("Start", |s| {
                    let name = s.call_on_name("capture_name", |view: &mut EditView| {
                        view.get_content()
                    }).unwrap();
                    s.pop_layer();
                    start_capture(s, name.to_string());
                })
                .button("Cancel", |s| { s.pop_layer(); })
                .with_name("capture_dialog")
        );
    } else if CAPTURE_IN_PROGRESS.load(Ordering::Relaxed) {
        if end {
            end_capture(s);
        } else {
            continue_capture(s);
        }
    }
}

fn start_capture(s: &mut cursive::Cursive, name: String) {
    let capture_state = CAPTURE_STATE.get().unwrap();
    let mut capture_state = capture_state.lock().unwrap();
    let servo = s.user_data::<Arc<Servo>>().unwrap();

    let filename = format!("cap-{}-{}.json", name, Local::now().format("%Y%m%d-%H%M%S"));
    capture_state.file = Some(File::create(&filename).unwrap());
    capture_state.captures.clear();
    capture_state.name = name;
    CAPTURE_IN_PROGRESS.store(true, Ordering::Relaxed);

    match servo.read_continuous() {
        Ok(data) => {
            let positions: serde_json::Map<String, Value> = data.servo
                .iter()
                .enumerate()
                .map(|(i, info)| ((i + 1).to_string(), json!(info.current_location)))
                .collect();
            
            capture_state.captures.push(json!({
                "pos": positions,
                "delay": 100
            }));
            
            s.add_layer(Dialog::info(format!("Started capture: {}. First position recorded.", filename)));
        }
        Err(e) => {
            s.add_layer(Dialog::info(format!("Error starting capture: {}", e)));
            CAPTURE_IN_PROGRESS.store(false, Ordering::Relaxed);
            capture_state.file = None;
        }
    }
}

fn continue_capture(s: &mut cursive::Cursive) {
    let capture_state = CAPTURE_STATE.get().unwrap();
    let mut capture_state = capture_state.lock().unwrap();
    let servo = s.user_data::<Arc<Servo>>().unwrap();

    match servo.read_continuous() {
        Ok(data) => {
            let positions: serde_json::Map<String, Value> = data.servo
                .iter()
                .enumerate()
                .map(|(i, info)| ((i + 1).to_string(), json!(info.current_location)))
                .collect();
            
            capture_state.captures.push(json!({
                "pos": positions,
                "delay": 100
            }));
            
            s.add_layer(Dialog::info("Position captured"));
        }
        Err(e) => {
            s.add_layer(Dialog::info(format!("Error capturing position: {}", e)));
        }
    }
}

fn end_capture(s: &mut cursive::Cursive) {
    let capture_state = CAPTURE_STATE.get().unwrap();
    let mut capture_state = capture_state.lock().unwrap();

    if let Some(mut file) = capture_state.file.take() {
        let captures = capture_state.captures.clone();
        let json = json!({
            "name": capture_state.name,
            "cap": captures
        });
        write!(file, "{}", json.to_string()).unwrap();
        capture_state.captures.clear();
        CAPTURE_IN_PROGRESS.store(false, Ordering::Relaxed);
        s.add_layer(Dialog::info("Capture ended and saved"));
    }
}

// Add this function to set the torque for a servo
fn set_servo_torque(servo_id: u8, torque: u16, servo: &Arc<Servo>) -> Result<()> {
    // Unlock EEPROM
    servo.write(servo_id, ServoRegister::LockMark, &[0])?;
    std::thread::sleep(Duration::from_millis(10));

    // Write new torque limit
    servo.write_servo_memory(servo_id, ServoRegister::TorqueLimit, torque)?;
    std::thread::sleep(Duration::from_millis(10));

    // Lock EEPROM
    servo.write(servo_id, ServoRegister::LockMark, &[1])?;

    Ok(())
}