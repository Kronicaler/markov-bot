use druid::widget::{Controller, Flex, Label};
use druid::*;

pub const SET_MESSAGE_COUNT: Selector<usize> = Selector::new("message-count");

pub const ID_MESSAGE_COUNT: WidgetId = WidgetId::reserved(1);

#[derive(Clone, Data, Lens)]
pub struct FrontData {
    pub message_count: usize,
}

pub fn ui_builder() -> impl Widget<FrontData> {
    let label =
        Label::dynamic(|data: &usize, _env: &_| format!("Number of messages stored: {}", data))
            .controller(GeneralController)
            .with_id(ID_MESSAGE_COUNT)
            .lens(FrontData::message_count)
            .padding(5.0)
            .center();

    Flex::column().with_child(label)
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
                *data = cmd.get_unchecked(SET_MESSAGE_COUNT).clone();
            }
            _ => child.event(ctx, event, data, env),
        }
    }
}
