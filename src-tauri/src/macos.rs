//! AppKit owns the menu bar item, anchoring, popover chrome, and dismissal.
//! Tauri's hidden window only hosts the IPC-enabled webview during creation.
use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSApplication, NSAutoresizingMaskOptions, NSImage, NSPopover, NSPopoverBehavior, NSStatusBar,
    NSStatusItem, NSViewController, NSWindow,
};
use objc2_foundation::{
    ns_string, MainThreadMarker, NSObject, NSObjectProtocol, NSRectEdge, NSSize,
};
use std::cell::RefCell;
use tauri::{Emitter, Manager};

thread_local! { static MENU_BAR: RefCell<Option<MenuBar>> = const { RefCell::new(None) }; }

struct MenuBar {
    item: Retained<NSStatusItem>,
    popover: Retained<NSPopover>,
    _target: Retained<StatusTarget>,
}

define_class!(
    // NSObject has no subclass invariants. AppKit invokes the action on the main thread.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = tauri::AppHandle]
    struct StatusTarget;
    unsafe impl NSObjectProtocol for StatusTarget {}
    impl StatusTarget {
        #[unsafe(method(togglePopover:))]
        fn toggle_popover(&self, _sender: Option<&NSObject>) {
            MENU_BAR.with(|state| {
                if let Some(menu) = state.borrow().as_ref() {
                    if menu.popover.isShown() { menu.popover.close(); }
                    else { menu.show(self.mtm()); let _ = self.ivars().emit("panel-open", ()); }
                }
            });
        }
    }
);

impl MenuBar {
    fn show(&self, mtm: MainThreadMarker) {
        if let Some(button) = self.item.button(mtm) {
            // Activate the accessory app to allow keyboard input inside the popover.
            #[allow(deprecated)]
            NSApplication::sharedApplication(mtm).activateIgnoringOtherApps(true);
            self.popover.showRelativeToRect_ofView_preferredEdge(
                button.bounds(),
                &button,
                NSRectEdge::MinY,
            );
            if let Some(window) = self
                .popover
                .contentViewController()
                .and_then(|c| c.view().window())
            {
                window.makeKeyWindow();
            }
        }
    }
}

pub fn install(app: &tauri::AppHandle) -> tauri::Result<()> {
    let mtm = MainThreadMarker::new().expect("AppKit setup runs on the main thread");
    let window = app.get_webview_window("main").expect("main webview exists");
    let native_window = window.ns_window()?;
    // These objects are owned by Tauri and are only accessed on the main thread.
    let native_window = unsafe { &*native_window.cast::<NSWindow>() };
    let view = native_window
        .contentView()
        .expect("Tauri content view exists");
    native_window.orderOut(None);
    native_window.setExcludedFromWindowsMenu(true);
    native_window.setContentView(None);
    view.removeFromSuperview();
    view.setFrameSize(NSSize::new(360.0, 460.0));
    view.setAutoresizingMask(
        NSAutoresizingMaskOptions::ViewWidthSizable | NSAutoresizingMaskOptions::ViewHeightSizable,
    );
    let controller = NSViewController::new(mtm);
    controller.setView(&view);
    let popover = NSPopover::new(mtm);
    popover.setBehavior(NSPopoverBehavior::Transient);
    popover.setAnimates(true);
    popover.setContentViewController(Some(&controller));
    popover.setContentSize(NSSize::new(360.0, 460.0));
    let item = NSStatusBar::systemStatusBar().statusItemWithLength(28.0);
    let button = item.button(mtm).expect("status item has a button");
    if let Some(image) = NSImage::imageWithSystemSymbolName_accessibilityDescription(
        ns_string!("arrow.down.circle"),
        Some(ns_string!("YT Down")),
    ) {
        image.setTemplate(true);
        image.setSize(NSSize::new(18.0, 18.0));
        button.setImage(Some(&image));
    }
    button.setToolTip(Some(ns_string!("YT Down")));
    let target = StatusTarget::alloc(mtm).set_ivars(app.clone());
    let target: Retained<StatusTarget> = unsafe { msg_send![super(target), init] };
    // The target is retained in MENU_BAR for the lifetime of the status item.
    unsafe {
        button.setTarget(Some(&target));
        button.setAction(Some(sel!(togglePopover:)));
    }
    MENU_BAR.with(|state| {
        *state.borrow_mut() = Some(MenuBar {
            item,
            popover,
            _target: target,
        })
    });
    Ok(())
}

pub fn show(app: &tauri::AppHandle) {
    present(app, false);
}

pub fn restore(app: &tauri::AppHandle) {
    present(app, true);
}

fn present(app: &tauri::AppHandle, preserve_view: bool) {
    let app = app.clone();
    let dispatcher = app.clone();
    let _ = dispatcher.run_on_main_thread(move || {
        MENU_BAR.with(|state| {
            if let Some(menu) = state.borrow().as_ref() {
                menu.show(MainThreadMarker::new().unwrap());
                let _ = app.emit(
                    "panel-open",
                    serde_json::json!({ "preserveView": preserve_view }),
                );
            }
        });
    });
}

pub fn close(app: &tauri::AppHandle) {
    let _ = app.run_on_main_thread(|| {
        MENU_BAR.with(|state| {
            if let Some(menu) = state.borrow().as_ref() {
                menu.popover.close();
            }
        })
    });
}

pub fn resize(app: &tauri::AppHandle, height: f64) {
    let height = height.clamp(260.0, 600.0);
    let _ = app.run_on_main_thread(move || {
        MENU_BAR.with(|state| {
            if let Some(menu) = state.borrow().as_ref() {
                menu.popover.setContentSize(NSSize::new(360.0, height));
            }
        })
    });
}

pub fn request_notification_permission(app: &tauri::AppHandle) {
    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_foundation::NSError;
    use objc2_user_notifications::{UNAuthorizationOptions, UNUserNotificationCenter};
    let app = app.clone();
    let completion = RcBlock::new(move |_granted: Bool, error: *mut NSError| {
        if !error.is_null() {
            let _ = app.emit("app-error", "macOS could not enable notifications. Check System Settings → Notifications → YT Down.");
        }
    });
    UNUserNotificationCenter::currentNotificationCenter()
        .requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert,
            &completion,
        );
}

pub fn notify_missing(app: &tauri::AppHandle, message: String) {
    use block2::RcBlock;
    use objc2::runtime::Bool;
    use objc2_foundation::{NSError, NSString};
    use objc2_user_notifications::{
        UNAuthorizationOptions, UNMutableNotificationContent, UNNotificationRequest,
        UNUserNotificationCenter,
    };
    let app = app.clone();
    let completion = RcBlock::new(move |granted: Bool, error: *mut NSError| {
        if !granted.as_bool() || !error.is_null() {
            let _ = app.emit("app-error", format!("{message} Enable YT Down notifications in System Settings to receive system alerts."));
            return;
        }
        let content = UNMutableNotificationContent::new();
        content.setTitle(ns_string!("YT Down needs a dependency"));
        content.setBody(&NSString::from_str(&message));
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            ns_string!("missing-download-tools"),
            &content,
            None,
        );
        let report_app = app.clone();
        let delivered = RcBlock::new(move |error: *mut NSError| {
            if !error.is_null() {
                let _ = report_app.emit(
                    "app-error",
                    "macOS could not deliver the missing-tool notification.",
                );
            }
        });
        UNUserNotificationCenter::currentNotificationCenter()
            .addNotificationRequest_withCompletionHandler(&request, Some(&delivered));
    });
    UNUserNotificationCenter::currentNotificationCenter()
        .requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert,
            &completion,
        );
}
