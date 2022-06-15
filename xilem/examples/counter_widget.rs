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
    pub fn new() -> Self {
        let id_path = IdPath::new(); // Needs a value.
        let widgets = vec![
            Pod::new(TextWidget::new(format!("Count: {}", 0))),
            Pod::new(widget::button::Button::new(
                &id_path,
                "Increase".to_string(),
            )),
        ];
        CounterWidget {
            count: 0,
            stack: widget::vstack::VStack::new(widgets),
        }
    }
}

impl Widget for CounterWidget {
    fn event(&mut self, cx: &mut EventCx, event: &RawEvent) {
        let button_clicked = false; // Need a callback for this

        if button_clicked {
            self.count += 1;

            let label: &mut TextWidget = self.stack.children_mut()[0].downcast_mut().unwrap();
            label.set_text(format!("Count: {}", self.count));
            // label.update(cx); - Cannot call
        }

        self.stack.event(cx, event);
    }

    fn lifecycle(&mut self, cx: &mut LifeCycleCx, event: &LifeCycle) {
        self.stack.lifecycle(cx, event);
    }

    fn update(&mut self, cx: &mut UpdateCx) {
        self.stack.update(cx);
    }

    fn measure(&mut self, cx: &mut LayoutCx) -> (Size, Size) {
        self.stack.measure(cx)
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
    type State = ();

    type Element = CounterWidget;

    fn build(&self, cx: &mut Cx) -> (Id, Self::State, Self::Element) {
        let (id, element) = cx.with_new_id(|_| CounterWidget::new());
        (id, (), element)
    }

    fn rebuild(
        &self,
        _cx: &mut Cx,
        _prev: &Self,
        _id: &mut Id,
        _state: &mut Self::State,
        _element: &mut Self::Element,
    ) -> bool {
        false
    }

    fn event(
        &self,
        _id_path: &[Id],
        _state: &mut Self::State,
        _event: Box<dyn Any>,
        _app_state: &mut (),
    ) -> EventResult<()> {
        EventResult::Nop
    }
}

pub fn main() {
    let app = App::new((), |_| CounterWidgetView);
    AppLauncher::new(app).run();
}
