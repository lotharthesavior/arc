import { Controller } from "@hotwired/stimulus";

/**
 * Active link controller
 * Highlights navigation links that match the current URL
 */
export default class extends Controller {
    static values = {
        path: String,
        exact: { type: Boolean, default: false },
        activeClass: { type: String, default: "bg-gray-50 dark:bg-gray-700 text-indigo-600 dark:text-indigo-400" },
    };

    connect() {
        this.updateActiveState();

        // Listen for Turbo navigation events
        document.addEventListener("turbo:load", this.updateActiveState.bind(this));
    }

    disconnect() {
        document.removeEventListener("turbo:load", this.updateActiveState.bind(this));
    }

    /**
     * Check if this link is active and apply classes
     */
    updateActiveState() {
        const currentPath = window.location.pathname;
        const linkPath = this.pathValue || this.element.getAttribute("href");

        let isActive = false;

        if (this.exactValue) {
            // Exact match
            isActive = currentPath === linkPath;
        } else {
            // Path starts with link path
            isActive = currentPath === linkPath ||
                      (linkPath !== "/" && currentPath.startsWith(linkPath));
        }

        this.toggleActiveClasses(isActive);
    }

    /**
     * Toggle active classes on the element
     * @param {boolean} isActive
     */
    toggleActiveClasses(isActive) {
        const classes = this.activeClassValue.split(" ");

        if (isActive) {
            classes.forEach((cls) => {
                if (cls) this.element.classList.add(cls);
            });
        } else {
            classes.forEach((cls) => {
                if (cls) this.element.classList.remove(cls);
            });
        }
    }
}
