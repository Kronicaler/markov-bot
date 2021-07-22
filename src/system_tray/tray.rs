extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use std::time::Duration;

use crossbeam::channel::{Receiver, Sender};
use nwd::NwgUi;
use nwg::NativeUi;

#[derive(Default, NwgUi)]
pub struct SystemTray {
    #[nwg_control]
    window: nwg::MessageWindow,

    #[nwg_resource(source_file: Some("src/system_tray/doki.ico"))]
    icon: nwg::Icon,

    #[nwg_control(icon: Some(&data.icon), tip: Some("Doki Bot"))]
    #[nwg_events(MousePressLeftUp: [SystemTray::show_menu], OnContextMenu: [SystemTray::show_menu])]
    tray: nwg::TrayNotification,

    #[nwg_control(parent: window, popup: true)]
    tray_menu: nwg::Menu,

    #[nwg_control(parent: tray_menu, text: "Hello")]
    #[nwg_events(OnMenuItemSelected: [SystemTray::hello1])]
    tray_item1: nwg::MenuItem,

    #[nwg_control(parent: tray_menu, text: "Popup")]
    #[nwg_events(OnMenuItemSelected: [SystemTray::hello2])]
    tray_item2: nwg::MenuItem,

    #[nwg_control(parent: tray_menu, text: "Exit")]
    #[nwg_events(OnMenuItemSelected: [SystemTray::exit])]
    tray_item3: nwg::MenuItem,
}

impl SystemTray {
    fn show_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.tray_menu.popup(x, y);
    }

    fn hello1(&self) {
        nwg::simple_message("Hello", "Hello World!");
    }

    fn hello2(&self) {
        let flags = nwg::TrayNotificationFlags::USER_ICON | nwg::TrayNotificationFlags::LARGE_ICON;
        self.tray.show(
            "Hello World",
            Some("Welcome to my application"),
            Some(flags),
            Some(&self.icon),
        );
    }

    fn exit(&self) {
        nwg::stop_thread_dispatch();
    }
}

pub fn create_tray_icon(tray_thread_quit_sender: Sender<bool>) {
    nwg::init().expect("Failed to init Native Windows GUI");
    let _ui = SystemTray::build_ui(SystemTray::default()).expect("Failed to build UI");
    nwg::dispatch_thread_events();
    tray_thread_quit_sender.send(true).unwrap();
}

pub async fn start_tray(export_and_quit_receiver_tray: Receiver<bool>) {
    let (tray_thread_quit_sender, tray_thread_quit_receiver): (Sender<bool>, Receiver<bool>) =
        crossbeam::channel::unbounded();
    std::thread::spawn(|| create_tray_icon(tray_thread_quit_sender));

    loop {
        if let Ok(_) = export_and_quit_receiver_tray.try_recv() {
            return;
        }
        if let Ok(_) = tray_thread_quit_receiver.try_recv() {
            return;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
