import protobuf from 'protobufjs'

// Inline proto definition for portability (no filesystem access needed)
const INPUT_PROTO_TEXT = `
syntax = "proto3";
package remote_work;

message InputEvent {
  oneof event {
    MouseMove mouse_move = 1;
    MouseButton mouse_button = 2;
    MouseScroll mouse_scroll = 3;
    KeyEvent key_event = 4;
  }
}

message MouseMove {
  float x = 1;
  float y = 2;
}

message MouseButton {
  enum Button {
    LEFT = 0;
    RIGHT = 1;
    MIDDLE = 2;
  }
  Button button = 1;
  bool pressed = 2;
  float x = 3;
  float y = 4;
}

message MouseScroll {
  float delta_x = 1;
  float delta_y = 2;
}

message KeyEvent {
  uint32 key_code = 1;
  bool pressed = 2;
  uint32 modifiers = 3;
}
`

// Parse once at module load time (synchronous)
const root = protobuf.parse(INPUT_PROTO_TEXT).root
const InputEventType = root.lookupType('remote_work.InputEvent')

/** Map browser KeyboardEvent.code to USB HID keycodes */
function keyCodeToHid(code: string): number {
  const map: Record<string, number> = {
    KeyA: 0x04, KeyB: 0x05, KeyC: 0x06, KeyD: 0x07, KeyE: 0x08,
    KeyF: 0x09, KeyG: 0x0a, KeyH: 0x0b, KeyI: 0x0c, KeyJ: 0x0d,
    KeyK: 0x0e, KeyL: 0x0f, KeyM: 0x10, KeyN: 0x11, KeyO: 0x12,
    KeyP: 0x13, KeyQ: 0x14, KeyR: 0x15, KeyS: 0x16, KeyT: 0x17,
    KeyU: 0x18, KeyV: 0x19, KeyW: 0x1a, KeyX: 0x1b, KeyY: 0x1c,
    KeyZ: 0x1d,
    Digit1: 0x1e, Digit2: 0x1f, Digit3: 0x20, Digit4: 0x21, Digit5: 0x22,
    Digit6: 0x23, Digit7: 0x24, Digit8: 0x25, Digit9: 0x26, Digit0: 0x27,
    Enter: 0x28, Escape: 0x29, Backspace: 0x2a, Tab: 0x2b, Space: 0x2c,
    Minus: 0x2d, Equal: 0x2e,
    ArrowRight: 0x4f, ArrowLeft: 0x50, ArrowDown: 0x51, ArrowUp: 0x52,
    Home: 0x4a, End: 0x4d, PageUp: 0x4b, PageDown: 0x4e, Delete: 0x4c,
    ControlLeft: 0xe0, ShiftLeft: 0xe1, AltLeft: 0xe2, MetaLeft: 0xe3,
    ControlRight: 0xe4, ShiftRight: 0xe5, AltRight: 0xe6, MetaRight: 0xe7,
    F1: 0x3a, F2: 0x3b, F3: 0x3c, F4: 0x3d, F5: 0x3e, F6: 0x3f,
    F7: 0x40, F8: 0x41, F9: 0x42, F10: 0x43, F11: 0x44, F12: 0x45,
  }
  return map[code] ?? 0
}

function encodeInputEvent(payload: object): ArrayBuffer | null {
  const err = InputEventType.verify(payload)
  if (err) {
    console.warn('[InputSender] proto verify:', err)
    return null
  }
  const msg = InputEventType.create(payload)
  const buf = InputEventType.encode(msg).finish()
  // Return a copy of the underlying buffer slice
  return buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength) as ArrayBuffer
}

export class InputSender {
  private element: HTMLElement | null = null
  private listeners: Array<{ target: EventTarget; type: string; fn: EventListener }> = []

  constructor(private sendFn: (data: ArrayBuffer) => void) {}

  attach(element: HTMLElement): void {
    this.element = element

    this.on(element, 'mousemove', (e) => this.handleMouseMove(e as MouseEvent))
    this.on(element, 'mousedown', (e) => this.handleMouseButton(e as MouseEvent, true))
    this.on(element, 'mouseup', (e) => this.handleMouseButton(e as MouseEvent, false))
    this.on(element, 'wheel', (e) => this.handleWheel(e as WheelEvent), { passive: false })
    this.on(element, 'contextmenu', (e) => e.preventDefault())

    // Keyboard events are captured globally so focus stays on the container
    this.on(window, 'keydown', (e) => this.handleKey(e as KeyboardEvent, true))
    this.on(window, 'keyup', (e) => this.handleKey(e as KeyboardEvent, false))
  }

  detach(): void {
    for (const { target, type, fn } of this.listeners) {
      target.removeEventListener(type, fn)
    }
    this.listeners = []
    this.element = null
  }

  private on(
    target: EventTarget,
    type: string,
    fn: EventListener,
    opts?: AddEventListenerOptions,
  ): void {
    target.addEventListener(type, fn, opts)
    this.listeners.push({ target, type, fn })
  }

  private normalizedCoords(e: MouseEvent): { x: number; y: number } {
    const rect = this.element!.getBoundingClientRect()
    return {
      x: Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width)),
      y: Math.max(0, Math.min(1, (e.clientY - rect.top) / rect.height)),
    }
  }

  private handleMouseMove(e: MouseEvent): void {
    const { x, y } = this.normalizedCoords(e)
    const data = encodeInputEvent({ mouseMove: { x, y } })
    if (data) this.sendFn(data)
  }

  private handleMouseButton(e: MouseEvent, pressed: boolean): void {
    e.preventDefault()
    const { x, y } = this.normalizedCoords(e)
    // Map browser button index → proto Button enum (LEFT=0, RIGHT=1, MIDDLE=2)
    const button = e.button === 2 ? 1 : e.button === 1 ? 2 : 0
    const data = encodeInputEvent({ mouseButton: { button, pressed, x, y } })
    if (data) this.sendFn(data)
  }

  private handleWheel(e: WheelEvent): void {
    e.preventDefault()
    // Normalise delta: clamp to a reasonable range in "scroll units"
    const clamp = (v: number) => Math.sign(v) * Math.min(Math.abs(v) / 100, 10)
    const data = encodeInputEvent({
      mouseScroll: { deltaX: clamp(e.deltaX), deltaY: clamp(e.deltaY) },
    })
    if (data) this.sendFn(data)
  }

  private handleKey(e: KeyboardEvent, pressed: boolean): void {
    const keyCode = keyCodeToHid(e.code)
    if (keyCode === 0) return

    // Prevent browser shortcuts from interfering (Ctrl+R, Ctrl+W, etc.)
    if (e.ctrlKey || e.metaKey || e.altKey) e.preventDefault()

    const modifiers =
      (e.ctrlKey ? 1 : 0) |
      (e.shiftKey ? 2 : 0) |
      (e.altKey ? 4 : 0) |
      (e.metaKey ? 8 : 0)

    const data = encodeInputEvent({ keyEvent: { keyCode, pressed, modifiers } })
    if (data) this.sendFn(data)
  }
}
