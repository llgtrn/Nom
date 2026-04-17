export type NotificationType = "success" | "error" | "warning" | "info";

export interface Notification {
  id: string;
  type: NotificationType;
  message: string;
  duration: number;
}

export class NotificationManager {
  private container: HTMLElement;
  private nextId = 0;

  constructor() {
    this.container = document.createElement("div");
    this.container.className = "notification-container";
    document.body.appendChild(this.container);
  }

  show(message: string, type: NotificationType = "info", duration = 3000): string {
    const id = `notif-${this.nextId++}`;
    const el = document.createElement("div");
    el.className = `notification notification-${type}`;
    el.id = id;

    const icon = document.createElement("span");
    icon.className = "notification-icon";
    icon.textContent = type === "success" ? "OK" : type === "error" ? "!!" : type === "warning" ? "!" : "i";
    el.appendChild(icon);

    const text = document.createElement("span");
    text.className = "notification-text";
    text.textContent = message;
    el.appendChild(text);

    const close = document.createElement("button");
    close.className = "notification-close";
    close.textContent = "x";
    close.addEventListener("click", () => this.dismiss(id));
    el.appendChild(close);

    this.container.appendChild(el);

    // Auto-dismiss
    if (duration > 0) {
      setTimeout(() => this.dismiss(id), duration);
    }

    return id;
  }

  dismiss(id: string): void {
    const el = document.getElementById(id);
    if (el) {
      el.classList.add("notification-exit");
      setTimeout(() => el.remove(), 300);
    }
  }

  success(msg: string): string { return this.show(msg, "success"); }
  error(msg: string): string { return this.show(msg, "error", 5000); }
  warning(msg: string): string { return this.show(msg, "warning"); }
  info(msg: string): string { return this.show(msg, "info"); }
}
