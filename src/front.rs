use std::sync::mpsc::Sender;

use druid::widget::{Button, Controller, Flex, Label};
use druid::*;

pub const SET_MESSAGE_COUNT: Selector<usize> = Selector::new("message-count");

pub const ID_MESSAGE_COUNT: WidgetId = WidgetId::reserved(1);

#[derive(Clone, Data, Lens)]
pub struct GuiData {
    pub message_count: usize,
    #[data(ignore)]
    pub quit_sender: Sender<bool>,
}

pub async fn start_gui(tx: Sender<ExtEventSink>, quit_sender: Sender<bool>) {
    let window = WindowDesc::new(ui_builder)
        .title("Doki Bot")
        .window_size((50.0, 50.0));
    let launcher = AppLauncher::with_window(window);
    tx.send(launcher.get_external_handle()).unwrap();
    let data: GuiData = GuiData {
        message_count: 0,
        quit_sender,
    };
    launcher.launch(data).unwrap();
}

pub fn ui_builder() -> impl Widget<GuiData> {
    let msg_count_label =
        Label::dynamic(|data: &usize, _env: &_| format!("Number of messages stored: {}", data))
            .controller(GeneralController)
            .with_id(ID_MESSAGE_COUNT)
            .lens(GuiData::message_count)
            .padding(5.0)
            .center();

    let quit_button = Button::new("Quit client").on_click(|ctx, data: &mut GuiData, _env| {
        {
            data.quit_sender.send(true).unwrap();
        }
        ctx.window().close();
    });

    Flex::column()
        .with_child(msg_count_label)
        .with_child(quit_button)
}

struct GeneralController;

impl Controller<usize, Label<usize>> for GeneralController {
    fn event(
        &mut self,
        child: &mut Label<usize>,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut usize,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(SET_MESSAGE_COUNT) => {
                *data = *cmd.get_unchecked(SET_MESSAGE_COUNT);
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}
