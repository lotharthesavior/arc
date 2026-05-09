import * as Turbo from "@hotwired/turbo";
import { Application } from "@hotwired/stimulus";
import Toastify from "toastify-js";
import "toastify-js/src/toastify.css";

// Initialize Stimulus application
const application = Application.start();

// Import controllers
import DarkModeController from "./controllers/dark_mode_controller";
import DropdownController from "./controllers/dropdown_controller";
import MobileMenuController from "./controllers/mobile_menu_controller";
import NotificationController from "./controllers/notification_controller";
import SessionNotificationController from "./controllers/session_notification_controller";
import ProfileFormController from "./controllers/profile_form_controller";
import PasswordFormController from "./controllers/password_form_controller";
import ActiveLinkController from "./controllers/active_link_controller";
import WebSocketController from "./controllers/websocket_controller";

// Register controllers
application.register("dark-mode", DarkModeController);
application.register("dropdown", DropdownController);
application.register("mobile-menu", MobileMenuController);
application.register("notification", NotificationController);
application.register("session-notification", SessionNotificationController);
application.register("profile-form", ProfileFormController);
application.register("password-form", PasswordFormController);
application.register("active-link", ActiveLinkController);
application.register("websocket", WebSocketController);

// Make Toastify available globally for notifications
window.Toastify = Toastify;

// Configure Turbo
Turbo.setProgressBarDelay(100);
