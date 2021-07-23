use crate::*;
use druid::widget::{Button, Controller, Flex, Label};
use druid::*;

pub const SET_MESSAGE_COUNT: Selector<usize> = Selector::new("message-count");

pub const ID_MESSAGE_COUNT: WidgetId = WidgetId::reserved(1);

#[derive(Clone, Data, Lens)]
pub struct GuiData {
    pub message_count: usize,
    #[data(ignore)]
    pub senders_to_client: SendersToClient,
}

#[derive(Clone)]
pub struct SendersToClient {
    pub export_and_quit: Sender<bool>,
}

pub fn start_gui(tx: &Sender<ExtEventSink>, senders_to_client: SendersToClient) {
    let window = WindowDesc::new(ui_builder)
        .title("Doki Bot")
        .window_size((50.0, 50.0));
    let launcher = AppLauncher::with_window(window);
    tx.send(launcher.get_external_handle()).unwrap();
    let data: GuiData = GuiData {
        message_count: 0,
        senders_to_client,
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

    let export_button = Button::new("Export Markov Chain and close the Client").on_click(
        |_ctx, data: &mut GuiData, _env| {
            data.senders_to_client
                .export_and_quit
                .try_send(true)
                .unwrap();
        },
    );

    Flex::column()
        .with_child(msg_count_label)
        .with_child(export_button)
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
