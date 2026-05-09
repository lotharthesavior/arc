import { Controller } from "@hotwired/stimulus";
import * as Turbo from "@hotwired/turbo";

/**
 * WebSocket controller
 * Manages WebSocket connection for Turbo Streams
 */
export default class extends Controller {
    static values = {
        url: { type: String, default: "/ws" },
        reconnectDelay: { type: Number, default: 1000 },
        maxReconnectDelay: { type: Number, default: 30000 },
    };

    connect() {
        this.reconnectAttempts = 0;
        this.connectWebSocket();
    }

    disconnect() {
        this.closeWebSocket();
    }

    /**
     * Establish WebSocket connection
     */
    connectWebSocket() {
        const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
        const url = `${protocol}//${window.location.host}${this.urlValue}`;

        this.socket = new WebSocket(url);

        this.socket.onopen = this.handleOpen.bind(this);
        this.socket.onmessage = this.handleMessage.bind(this);
        this.socket.onclose = this.handleClose.bind(this);
        this.socket.onerror = this.handleError.bind(this);
    }

    /**
     * Close WebSocket connection
     */
    closeWebSocket() {
        if (this.socket) {
            this.socket.close();
            this.socket = null;
        }
        if (this.reconnectTimeout) {
            clearTimeout(this.reconnectTimeout);
            this.reconnectTimeout = null;
        }
    }

    /**
     * Handle WebSocket open
     */
    handleOpen() {
        console.log("WebSocket connected");
        this.reconnectAttempts = 0;
        this.dispatchConnectionEvent("connected");
    }

    /**
     * Handle incoming WebSocket messages
     * @param {MessageEvent} event
     */
    handleMessage(event) {
        const data = event.data;

        // Check if this is a Turbo Stream
        if (data.includes("<turbo-stream")) {
            Turbo.renderStreamMessage(data);
        } else {
            // Handle other message types (JSON commands, etc.)
            try {
                const json = JSON.parse(data);
                this.handleJsonMessage(json);
            } catch (e) {
                // Not JSON, might be plain text
                console.log("WebSocket message:", data);
            }
        }
    }

    /**
     * Handle JSON messages
     * @param {Object} message
     */
    handleJsonMessage(message) {
        switch (message.type) {
            case "pong":
                // Heartbeat response
                break;
            case "subscribed":
                console.log(`Subscribed to room: ${message.room}`);
                break;
            case "unsubscribed":
                console.log(`Unsubscribed from room: ${message.room}`);
                break;
            default:
                console.log("WebSocket JSON message:", message);
        }
    }

    /**
     * Handle WebSocket close
     * @param {CloseEvent} event
     */
    handleClose(event) {
        console.log("WebSocket disconnected", event.code, event.reason);
        this.dispatchConnectionEvent("disconnected");

        // Attempt to reconnect with exponential backoff
        if (!event.wasClean) {
            this.scheduleReconnect();
        }
    }

    /**
     * Handle WebSocket error
     * @param {Event} event
     */
    handleError(event) {
        console.error("WebSocket error:", event);
        this.dispatchConnectionEvent("error");
    }

    /**
     * Schedule a reconnection attempt
     */
    scheduleReconnect() {
        const delay = Math.min(
            this.reconnectDelayValue * Math.pow(2, this.reconnectAttempts),
            this.maxReconnectDelayValue
        );

        this.reconnectAttempts++;

        console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);

        this.reconnectTimeout = setTimeout(() => {
            this.connectWebSocket();
        }, delay);
    }

    /**
     * Subscribe to a room/channel
     * @param {string} room
     */
    subscribe(room) {
        this.send({ type: "subscribe", room });
    }

    /**
     * Unsubscribe from a room/channel
     * @param {string} room
     */
    unsubscribe(room) {
        this.send({ type: "unsubscribe", room });
    }

    /**
     * Send a message over WebSocket
     * @param {Object|string} data
     */
    send(data) {
        if (this.socket && this.socket.readyState === WebSocket.OPEN) {
            const message = typeof data === "string" ? data : JSON.stringify(data);
            this.socket.send(message);
        }
    }

    /**
     * Dispatch connection status event
     * @param {string} status
     */
    dispatchConnectionEvent(status) {
        window.dispatchEvent(
            new CustomEvent("websocket:status", {
                detail: { status },
            })
        );
    }

    /**
     * Action to subscribe to a room via data-action
     */
    subscribeAction(event) {
        const room = event.params.room;
        if (room) {
            this.subscribe(room);
        }
    }

    /**
     * Action to unsubscribe from a room via data-action
     */
    unsubscribeAction(event) {
        const room = event.params.room;
        if (room) {
            this.unsubscribe(room);
        }
    }
}
