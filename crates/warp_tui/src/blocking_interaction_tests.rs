use std::cell::Cell;
use std::rc::Rc;

use warpui::App;
use warpui_core::EntityId;

use super::TuiBlockingInteractionModel;

#[test]
fn activating_a_new_blocker_rejects_when_another_blocker_is_active() {
    App::test((), |mut app| async move {
        let model = app.add_model(TuiBlockingInteractionModel::new);
        let first = EntityId::new();
        let second = EntityId::new();

        assert_eq!(
            model.update(&mut app, |model, ctx| model.activate(first, ctx)),
            Ok(())
        );
        let error = model
            .update(&mut app, |model, ctx| model.activate(second, ctx))
            .expect_err("a second blocker is rejected");

        assert_eq!(error.blocker, first);
        assert_eq!(model.read(&app, |model, _| model.blocker()), Some(first));
        assert!(model.update(&mut app, |model, ctx| model.deactivate(first, ctx)));
        assert_eq!(
            model.update(&mut app, |model, ctx| model.activate(second, ctx)),
            Ok(())
        );
        assert_eq!(model.read(&app, |model, _| model.blocker()), Some(second));
    });
}

#[test]
fn stale_deactivation_cannot_clear_the_current_blocker() {
    App::test((), |mut app| async move {
        let model = app.add_model(TuiBlockingInteractionModel::new);
        let first = EntityId::new();
        let second = EntityId::new();
        let notifications = Rc::new(Cell::new(0));
        let notifications_for_subscription = notifications.clone();
        app.update(|ctx| {
            ctx.subscribe_to_model(&model, move |_, _, _| {
                notifications_for_subscription.set(notifications_for_subscription.get() + 1);
            });
        });

        model
            .update(&mut app, |model, ctx| model.activate(first, ctx))
            .expect("first blocker activates");
        assert!(model.update(&mut app, |model, ctx| model.deactivate(first, ctx)));
        model
            .update(&mut app, |model, ctx| model.activate(second, ctx))
            .expect("second blocker activates after the first deactivates");
        assert!(!model.update(&mut app, |model, ctx| model.deactivate(first, ctx)));

        assert_eq!(model.read(&app, |model, _| model.blocker()), Some(second));
        assert_eq!(notifications.get(), 3);

        assert!(model.update(&mut app, |model, ctx| model.deactivate(second, ctx)));
        assert!(!model.read(&app, |model, _| model.is_active()));
        assert_eq!(notifications.get(), 4);
    });
}
