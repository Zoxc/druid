use std::any::Any;

use druid_shell::kurbo::Size;
use xilem::{
    widget::{
        self, text::TextWidget, EventCx, LayoutCx, LifeCycle, LifeCycleCx, PaintCx, Pod, RawEvent,
        UpdateCx,
    },
    App, AppLauncher, Cx, EventResult, Id, IdPath, View, Widget,
};

struct CounterWidget {
    count: u32,
    stack: widget::vstack::VStack,
}

impl CounterWidget {
    pub fn new(cx: &mut Cx) -> Self {
        let widgets = cx
            .with_new_id(|cx| {
                vec![
                    Pod::new(TextWidget::new(format!("Count: {}", 0))),
                    Pod::new(widget::button::Button::new(
                        cx.id_path(),
                        "Increase".to_string(),
                    )),
                ]
            })
            .1;
        CounterWidget {
            count: 0,
            stack: widget::vstack::VStack::new(widgets),
        }
    }
}

impl Widget for CounterWidget {
    fn message(&mut self, id_path: &[Id], event: Box<dyn Any>) -> EventResult<()> {
        self.count += 1;

        let label = &mut self.stack.children_mut()[0];
        label.request_update();
        let label: &mut TextWidget = label.downcast_mut().unwrap();
        label.set_text(format!("Count: {}", self.count));
        EventResult::RequestUpdate
    }

    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        self.stack.event(cx, event);
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.stack.lifecycle(cx, event);
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        self.stack.update(cx);
    }

    fn prelayout(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        self.stack.prelayout(cx)
    }

    fn layout(&mut self, cx: &mut LayoutCx, proposed_size: Size) -> Size {
        self.stack.layout(cx, proposed_size)
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        self.stack.paint(cx);
    }
}

// The rest is just plumbing to run the widget

struct CounterWidgetView;

impl View<(), ()> for CounterWidgetView {
    type State = bool;

    type Element = CounterWidget;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|cx| CounterWidget::new(cx));
        (id, false, element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        state: &mut Self::State,
        _element: &mut Self::Element,
    ) -> bool {
        //println!("rebuild {}", state);
        std::mem::take(state)
    }

    fn event(
        &self,
        id_path: &[Id],
        state: &mut Self::State,
        element: &mut Self::Element,
        event: Box<dyn Any>,
        _app_state: &mut (),
    ) -> EventResult<()> {
        let result = element.message(id_path, event);
        if let EventResult::RequestUpdate = result {
            *state = true;
        }
        result
    }
}

pub fn main() {
    let app = App::new((), |_| CounterWidgetView);
    AppLauncher::new(app).run();
}
