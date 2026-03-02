import { Controller } from "@hotwired/stimulus";

/**
 * Dark mode controller
 * Manages theme toggling between light and dark modes
 */
export default class extends Controller {
    static targets = ["sunIcon", "moonIcon"];

    connect() {
        this.initializeTheme();
    }

    /**
     * Initialize theme from localStorage or system preference
     */
    initializeTheme() {
        if (
            localStorage.theme === "dark" ||
            (!("theme" in localStorage) &&
                window.matchMedia("(prefers-color-scheme: dark)").matches)
        ) {
            document.documentElement.classList.add("dark");
            this.darkMode = true;
        } else {
            document.documentElement.classList.remove("dark");
            this.darkMode = false;
        }
        this.updateIcons();
    }

    /**
     * Toggle between light and dark mode
     */
    toggle() {
        this.darkMode = !this.darkMode;

        if (this.darkMode) {
            document.documentElement.classList.add("dark");
            localStorage.theme = "dark";
        } else {
            document.documentElement.classList.remove("dark");
            localStorage.theme = "light";
        }

        this.updateIcons();
    }

    /**
     * Update icon visibility based on current theme
     */
    updateIcons() {
        if (this.hasSunIconTarget && this.hasMoonIconTarget) {
            if (this.darkMode) {
                this.sunIconTarget.classList.remove("hidden");
                this.moonIconTarget.classList.add("hidden");
            } else {
                this.sunIconTarget.classList.add("hidden");
                this.moonIconTarget.classList.remove("hidden");
            }
        }
    }

    get darkMode() {
        return this._darkMode || false;
    }

    set darkMode(value) {
        this._darkMode = value;
    }
}
