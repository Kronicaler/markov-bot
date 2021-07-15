use std::sync::{Arc, Mutex};

use druid::widget::{Button, Controller, Flex, Label};
use druid::*;

pub const SET_MESSAGE_COUNT: Selector<usize> = Selector::new("message-count");

pub const ID_MESSAGE_COUNT: WidgetId = WidgetId::reserved(1);

#[derive(Clone, Data, Lens)]
pub struct FrontData {
    pub message_count: usize,
    pub should_quit_lock: Arc<Mutex<bool>>,
}

pub fn ui_builder() -> impl Widget<FrontData> {
    let msg_count_label =
        Label::dynamic(|data: &usize, _env: &_| format!("Number of messages stored: {}", data))
            .controller(GeneralController)
            .with_id(ID_MESSAGE_COUNT)
            .lens(FrontData::message_count)
            .padding(5.0)
            .center();

    let quit_button = Button::new("Quit client")
        .on_click(|_ctx, data: &mut Arc<Mutex<bool>>, _env| {
            {
                let mut should_quit = data.lock().unwrap();
                *should_quit = !*should_quit;
            }
            _ctx.window().close();
        })
        .lens(FrontData::should_quit_lock);

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
