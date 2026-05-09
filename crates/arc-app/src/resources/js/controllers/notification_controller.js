import { Controller } from "@hotwired/stimulus";

/**
 * Notification controller
 * Listens for custom notify events and displays toast notifications
 */
export default class extends Controller {
    connect() {
        this.boundHandleNotify = this.handleNotify.bind(this);
        window.addEventListener("notify", this.boundHandleNotify);
    }

    disconnect() {
        window.removeEventListener("notify", this.boundHandleNotify);
    }

    /**
     * Handle notify custom events
     * @param {CustomEvent} event
     */
    handleNotify(event) {
        const { message, type } = event.detail;
        this.showToast(message, type);
    }

    /**
     * Show a toast notification
     * @param {string} message
     * @param {string} type - success, error, info, warning
     */
    showToast(message, type = "info") {
        const backgroundColor = this.getBackgroundColor(type);

        window.Toastify({
            text: message,
            duration: 3000,
            close: true,
            gravity: "top",
            position: "right",
            stopOnFocus: true,
            style: {
                background: backgroundColor,
            },
        }).showToast();
    }

    /**
     * Get background color based on notification type
     * @param {string} type
     * @returns {string}
     */
    getBackgroundColor(type) {
        const colors = {
            success: "linear-gradient(to right, #00b09b, #96c93d)",
            error: "linear-gradient(to right, #ff5f6d, #ffc371)",
            warning: "linear-gradient(to right, #f7971e, #ffd200)",
            info: "linear-gradient(to right, #2193b0, #6dd5ed)",
        };
        return colors[type] || colors.info;
    }

    /**
     * Action to trigger a notification programmatically
     * Can be called from HTML with data-action
     */
    notify(event) {
        const message = event.params.message || "Notification";
        const type = event.params.type || "info";
        this.showToast(message, type);
    }
}
