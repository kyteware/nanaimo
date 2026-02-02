# Nanaimo "Auto" Area Implementation Roadmap

This roadmap breaks down the development into four logical phases, following the "Shell as Brain, Compositor as Muscle" architecture.

---

## Phase 1: Shell UI Foundation
*Goal: Build the Slint-based shell and establish the visual components.*

- [ ] **Shell Project Setup**: Create `nanaimo-shell` as a new Rust project with Slint dependencies
- [ ] **Slint UI Structure**: Design `.slint` files for:
  - Edge glow indicators
  - Sliding sidebar with window thumbnails
  - Text prompt input field
  - Progress/status display
- [ ] **Live Preview Workflow**: Configure Slint Preview for rapid iteration
- [ ] **Layer Shell Integration**: Use `wlr-layer-shell` to anchor sidebars to screen edges
- [ ] **Mock Mode**: Implement a standalone mode that runs on any Wayland/X11 desktop for UI testing
- [ ] **Basic Animations**: Implement slide-out, fade-in, and glow effects

---

## Phase 2: Protocol Implementation
*Goal: Establish the communication channel between compositor and shell.*

- [ ] **Protocol XML**: Create `ext-nanaimo-shell.xml` with all events and requests from the design
- [ ] **Code Generation**: Use `wayland-scanner` to generate Rust bindings
- [ ] **Compositor Integration**:
  - Add `ShellStateManager` to `NanaimoState`
  - Implement event emission for `window_drag_started`, `window_drag_updated`, `window_dropped`
  - Handle `define_zone`, `trap_window`, `release_window` requests
- [ ] **Shell Integration**:
  - Connect to `ext-nanaimo-shell-v1` on startup
  - Implement event handlers for window lifecycle and drag events
  - Implement zone definition and hit-testing logic
- [ ] **Multi-Monitor Support**: Query output geometry and define zones per-output
- [ ] **Edge Detection**: Implement logic to skip zones on edges that border other monitors

---

## Phase 3: MCP Integration
*Goal: Enable AI-assisted automation via the Model Context Protocol.*

- [ ] **MCP Protocol XML**: Create `ext-mcp.xml` for app communication
- [ ] **Compositor MCP Host**:
  - Add `McpManager` to `NanaimoState`
  - Implement tool invocation routing (shell → compositor → app)
  - Handle async tool execution without blocking the event loop
  - Emit `window_mcp_capabilities` when apps advertise tools
- [ ] **Shell MCP Orchestration**:
  - Integrate local LLM (Ollama, OpenAI API, or similar)
  - Parse natural language prompts into tool calls
  - Display reasoning progress via `mcp_tool_progress` events
  - Render tool results in the sidebar
- [ ] **Fallback Actions**: Implement basic compositor actions for non-MCP windows:
  - Screenshot, workspace move, resize, close, window info
- [ ] **Capability Indicators**: Add visual badges (✨) for MCP-capable windows

---

## Phase 4: Ecosystem & Polish
*Goal: Enable third-party adoption and refine the user experience.*

- [ ] **Reference MCP Client**: Build a small Rust/C library implementing `ext-mcp-v1`
- [ ] **Example App Integrations**:
  - Terminal wrapper that exposes "run command" tool
  - Text editor plugin for "search and replace" tool
  - Browser extension for "extract page content" tool
- [ ] **Documentation**:
  - Protocol specification for `ext-nanaimo-shell-v1` and `ext-mcp-v1`
  - Developer guide for adding MCP support to apps
  - User guide for the Auto Area feature
- [ ] **Performance Optimization**:
  - Profile drag event throughput
  - Optimize sidebar rendering for many trapped windows
  - Add window thumbnail caching
- [ ] **Keyboard Shortcuts**: Implement `Super+A` to send focused window to nearest Auto edge
- [ ] **Configuration**: Add user-configurable settings:
  - Auto Area width
  - LLM provider and API key
  - Edge glow color and intensity

---

## Phase 5 (Future): Advanced Features

- [ ] **Multi-Window MCP Sessions**: Allow apps to share context across multiple windows
- [ ] **Dock Integration**: Use the same zone system for a dock at the bottom edge
- [ ] **Workspace Switcher**: Edge zones for dragging windows between workspaces
- [ ] **Gesture Support**: Touchpad gestures to activate Auto mode
- [ ] **Voice Input**: Speak prompts instead of typing
