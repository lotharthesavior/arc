import { Controller } from "@hotwired/stimulus";

/**
 * Session notification controller
 * Displays session messages (like logout confirmation) as toast notifications on page load
 */
export default class extends Controller {
    static values = {
        message: String,
    };

    connect() {
        // Show session message if present
        if (this.messageValue && this.messageValue.trim().length > 0) {
            this.showToast(this.messageValue, "info");
        }
    }

    /**
     * Show a toast notification
     * @param {string} message
     * @param {string} type - success, error, info, warning
     */
    showToast(message, type = "info") {
        const typeColor = this.getTypeColor(type);

        window.Toastify({
            text: message,
            close: true,
            style: {
                background: typeColor,
                borderRadius: "6px",
            },
            gravity: "bottom",
            position: "right",
        }).showToast();
    }

    /**
     * Get color based on notification type
     * @param {string} type
     * @returns {string}
     */
    getTypeColor(type) {
        const colors = {
            success: "#16a34a",
            error: "#e11d48",
            warning: "#eab308",
            info: "#2563eb",
        };
        return colors[type] || colors.info;
    }
}
