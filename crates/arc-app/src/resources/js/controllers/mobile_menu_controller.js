import { Controller } from "@hotwired/stimulus";

/**
 * Mobile menu controller
 * Manages off-canvas sidebar menu for mobile devices
 */
export default class extends Controller {
    static targets = ["sidebar", "backdrop", "closeButton"];
    static values = {
        open: { type: Boolean, default: false },
    };

    connect() {
        this.boundKeydown = this.keydown.bind(this);
    }

    disconnect() {
        this.removeEventListeners();
        this.unlockBody();
    }

    /**
     * Open the mobile menu
     */
    open() {
        this.openValue = true;
    }

    /**
     * Close the mobile menu
     */
    close() {
        this.openValue = false;
    }

    /**
     * Toggle menu open/closed
     */
    toggle() {
        this.openValue = !this.openValue;
    }

    /**
     * Handle open value changes
     */
    openValueChanged() {
        if (this.openValue) {
            this.showMenu();
            this.lockBody();
            this.addEventListeners();
        } else {
            this.hideMenu();
            this.unlockBody();
            this.removeEventListeners();
        }
    }

    /**
     * Show the menu elements
     */
    showMenu() {
        if (this.hasSidebarTarget) {
            this.sidebarTarget.classList.remove("hidden");
        }
        if (this.hasBackdropTarget) {
            this.backdropTarget.classList.remove("hidden");
        }
    }

    /**
     * Hide the menu elements
     */
    hideMenu() {
        if (this.hasSidebarTarget) {
            this.sidebarTarget.classList.add("hidden");
        }
        if (this.hasBackdropTarget) {
            this.backdropTarget.classList.add("hidden");
        }
    }

    /**
     * Lock body scroll when menu is open
     */
    lockBody() {
        document.body.style.overflow = "hidden";
    }

    /**
     * Unlock body scroll when menu is closed
     */
    unlockBody() {
        document.body.style.overflow = "";
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
     * Add event listeners
     */
    addEventListeners() {
        document.addEventListener("keydown", this.boundKeydown);
    }

    /**
     * Remove event listeners
     */
    removeEventListeners() {
        document.removeEventListener("keydown", this.boundKeydown);
    }
}
