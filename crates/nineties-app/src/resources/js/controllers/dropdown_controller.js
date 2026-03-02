import { Controller } from "@hotwired/stimulus";

/**
 * Dropdown controller
 * Manages dropdown menu open/close state with click outside handling
 */
export default class extends Controller {
    static targets = ["menu"];
    static values = {
        open: { type: Boolean, default: false },
    };

    connect() {
        this.boundClickOutside = this.clickOutside.bind(this);
        this.boundKeydown = this.keydown.bind(this);
    }

    disconnect() {
        this.removeEventListeners();
    }

    /**
     * Toggle dropdown open/closed
     */
    toggle() {
        this.openValue = !this.openValue;
    }

    /**
     * Open the dropdown
     */
    open() {
        this.openValue = true;
    }

    /**
     * Close the dropdown
     */
    close() {
        this.openValue = false;
    }

    /**
     * Handle open value changes
     */
    openValueChanged() {
        if (this.hasMenuTarget) {
            if (this.openValue) {
                this.menuTarget.classList.remove("hidden");
                this.addEventListeners();
            } else {
                this.menuTarget.classList.add("hidden");
                this.removeEventListeners();
            }
        }
    }

    /**
     * Handle clicks outside the dropdown
     */
    clickOutside(event) {
        if (!this.element.contains(event.target)) {
            this.close();
        }
    }

    /**
     * Handle keyboard events
     */
    keydown(event) {
        if (event.key === "Escape") {
            this.close();
        }
    }

    /**
     * Add event listeners when dropdown is open
     */
    addEventListeners() {
        document.addEventListener("click", this.boundClickOutside);
        document.addEventListener("keydown", this.boundKeydown);
    }

    /**
     * Remove event listeners when dropdown is closed
     */
    removeEventListeners() {
        document.removeEventListener("click", this.boundClickOutside);
        document.removeEventListener("keydown", this.boundKeydown);
    }
}
