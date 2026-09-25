//! # OpenPet 3D Animated Companion & Chat Dock
//!
//! High-fidelity 3D animated companion dock inspired by modern desktop companions
//! (such as Furever Dock) and designed around the exact frosted capsule chat prompt
//! widget specification.
//!
//! Features:
//! - Real-time hardware-accelerated 3D WebGL rendering via embedded Three.js.
//! - Continuous 60 FPS skeletal/procedural animations:
//!   - Lifelike thoracic breathing cycle (chest and spine displacement).
//!   - Dynamic 3D head and eye tracking responding to desktop cursor movement.
//!   - Multi-joint segmented tail physics with harmonic oscillation.
//!   - Natural autonomic blinking cycle with articulated eyelids.
//!   - Paws resting naturally on the top edge of the frosted capsule bar.
//!   - Expressive state animations: High-five paw wave, arching stretch, cozy loaf sleep.
//! - Frosted capsule chat pill ("What you want to ask? ↵") with input field and enter trigger.
//! - Reactive comic speech bubbles anchored above the companion head.
//! - Seamless drag-and-drop desktop positioning with transparent alpha blending.

use openpet_types::CompanionArtStyle;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use tracing::{error, info};
use wry::WebViewBuilder;

/// Offline bundled Three.js library (r128).
const THREE_JS_LIB: &str = include_str!("../../../crates/openpet-render/assets/three.min.js");

/// Inter-thread commands sent from the host/engine to the 3D dock window.
#[derive(Debug, Clone)]
pub enum Dock3DCommand {
    /// Toggles visibility of the 3D dock window.
    Show(bool),
    /// Injects an assistant chat response into the floating speech bubble.
    ChatMessage { sender: String, text: String },
    /// Triggers an expressive 3D animation (e.g. "high_five", "sleep", "stretch", "purr").
    TriggerAnimation(String),
    /// Switches the active companion art style.
    SetArtStyle(CompanionArtStyle),
    /// Gracefully closes and terminates the 3D dock window.
    Close,
}

/// Outgoing events sent from the 3D dock window back to the host/behavior engine.
#[derive(Debug, Clone)]
pub enum Dock3DEvent {
    /// User submitted a query via the "What you want to ask? ↵" prompt bar.
    SubmitChat(String),
    /// User requested an interaction (feed, water, play, pet).
    Interact(String),
    /// User toggled the companion art style from the dock context menu.
    ToggleArtStyle,
    /// Dock was closed or dismissed by the user.
    Closed,
}

/// Handle to communicate with the running 3D dock window.
#[derive(Clone)]
pub struct Dock3DHandle {
    sender: Sender<Dock3DCommand>,
}

impl Dock3DHandle {
    /// Sends a command to the 3D dock window.
    pub fn send(&self, cmd: Dock3DCommand) -> Result<(), mpsc::SendError<Dock3DCommand>> {
        self.sender.send(cmd)
    }

    /// Injects a chat response to be displayed above the 3D companion.
    pub fn send_chat_response(&self, text: &str) {
        let _ = self.sender.send(Dock3DCommand::ChatMessage {
            sender: "Oreo".to_string(),
            text: text.to_string(),
        });
    }

    /// Triggers a 3D animation state.
    pub fn trigger_anim(&self, anim_name: &str) {
        let _ = self
            .sender
            .send(Dock3DCommand::TriggerAnimation(anim_name.to_string()));
    }
}

/// Spawns the 3D Animated Companion Dock window on a dedicated OS thread.
pub fn spawn_3d_dock_window(
    initial_visible: bool,
    event_sender: Sender<Dock3DEvent>,
) -> Result<Dock3DHandle, Box<dyn std::error::Error>> {
    let (cmd_tx, cmd_rx) = mpsc::channel::<Dock3DCommand>();
    let handle = Dock3DHandle {
        sender: cmd_tx.clone(),
    };

    thread::Builder::new()
        .name("openpet-3d-dock".to_string())
        .spawn(move || {
            if let Err(e) = run_3d_dock_event_loop(initial_visible, cmd_rx, event_sender) {
                error!("Failed to run 3D dock event loop: {}", e);
            }
        })?;

    Ok(handle)
}

/// Event loop and window runner for the 3D dock.
fn run_3d_dock_event_loop(
    initial_visible: bool,
    cmd_rx: Receiver<Dock3DCommand>,
    event_tx: Sender<Dock3DEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoopBuilder::new().build();

    let window = WindowBuilder::new()
        .with_title("OpenPet 3D Companion Dock")
        .with_transparent(true)
        .with_decorations(false)
        .with_always_on_top(true)
        .with_inner_size(LogicalSize::new(540.0, 500.0))
        .with_resizable(false)
        .with_visible(initial_visible)
        .build(&event_loop)?;

    let html_content = generate_dock_html();

    let drag_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let drag_flag_ipc = drag_flag.clone();
    let event_tx_clone = event_tx.clone();
    let webview = WebViewBuilder::new()
        .with_transparent(true)
        .with_ipc_handler(move |request| {
            let body = request.body();
            handle_js_ipc_message(body, &event_tx_clone, &drag_flag_ipc);
        })
        .with_html(html_content)
        .build(&window)?;

    info!("OpenPet 3D Companion & Chat Dock initialized successfully.");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::WaitUntil(
            std::time::Instant::now() + std::time::Duration::from_millis(16),
        );

        // Process incoming commands from host
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                Dock3DCommand::Show(visible) => {
                    window.set_visible(visible);
                }
                Dock3DCommand::ChatMessage { sender: _, text } => {
                    let escaped = text
                        .replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('\n', "\\n")
                        .replace('\r', "");
                    let script = format!("window.showChatBubble(\"{}\");", escaped);
                    let _ = webview.evaluate_script(&script);
                }
                Dock3DCommand::TriggerAnimation(anim) => {
                    let script = format!("window.triggerPetAnim(\"{}\");", anim);
                    let _ = webview.evaluate_script(&script);
                }
                Dock3DCommand::SetArtStyle(style) => {
                    let style_name = match style {
                        CompanionArtStyle::PixelArt => "pixel",
                        CompanionArtStyle::Realistic => "realistic",
                    };
                    let script = format!("window.setCompanionStyle(\"{}\");", style_name);
                    let _ = webview.evaluate_script(&script);
                }
                Dock3DCommand::Close => {
                    *control_flow = ControlFlow::Exit;
                }
            }
        }

        if drag_flag.swap(false, std::sync::atomic::Ordering::SeqCst) {
            let _ = window.drag_window();
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                let _ = event_tx.send(Dock3DEvent::Closed);
                *control_flow = ControlFlow::Exit;
            }
            Event::MainEventsCleared => {
                // Keep UI reactive
            }
            _ => (),
        }
    });
}

/// Dispatches JSON IPC requests sent from the WebView JavaScript layer.
fn handle_js_ipc_message(
    body: &str,
    event_tx: &Sender<Dock3DEvent>,
    drag_flag: &std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(msg_type) = val.get("type").and_then(|v| v.as_str()) {
            match msg_type {
                "drag" => {
                    drag_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                "chat" => {
                    if let Some(text) = val.get("text").and_then(|v| v.as_str()) {
                        let _ = event_tx.send(Dock3DEvent::SubmitChat(text.to_string()));
                    }
                }
                "interact" => {
                    if let Some(action) = val.get("action").and_then(|v| v.as_str()) {
                        let _ = event_tx.send(Dock3DEvent::Interact(action.to_string()));
                    }
                }
                "toggle_style" => {
                    let _ = event_tx.send(Dock3DEvent::ToggleArtStyle);
                }
                "close" => {
                    let _ = event_tx.send(Dock3DEvent::Closed);
                }
                _ => {}
            }
        }
    }
}

/// Generates the complete, self-contained HTML/CSS/JS document for the 3D dock.
fn generate_dock_html() -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>OpenPet 3D Dock</title>
<style>
  * {{
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    user-select: none;
    -webkit-user-select: none;
  }}

  body, html {{
    width: 100%;
    height: 100%;
    overflow: hidden;
    background: transparent !important;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  }}

  #root {{
    position: relative;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-end;
    padding-bottom: 24px;
    background: transparent;
  }}

  /* Comic Speech / Thought Bubble */
  #speech-bubble {{
    position: absolute;
    top: 24px;
    max-width: 380px;
    background: rgba(255, 252, 248, 0.96);
    backdrop-filter: blur(16px);
    border: 1.5px solid rgba(230, 215, 200, 0.85);
    border-radius: 20px;
    padding: 12px 18px;
    box-shadow: 0 12px 32px rgba(74, 62, 61, 0.15), 0 2px 6px rgba(0, 0, 0, 0.05);
    color: #4A3E3D;
    font-size: 14px;
    line-height: 1.45;
    opacity: 0;
    transform: translateY(12px) scale(0.96);
    transition: opacity 0.3s cubic-bezier(0.16, 1, 0.3, 1), transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
    pointer-events: auto;
    z-index: 100;
  }}

  #speech-bubble.visible {{
    opacity: 1;
    transform: translateY(0) scale(1);
  }}

  #speech-bubble::after {{
    content: '';
    position: absolute;
    bottom: -8px;
    left: 50%;
    transform: translateX(-50%);
    border-width: 8px 8px 0;
    border-style: solid;
    border-color: rgba(255, 252, 248, 0.96) transparent;
    display: block;
    width: 0;
  }}

  /* 3D WebGL Canvas Layer */
  #canvas-container {{
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 380px;
    pointer-events: auto;
    cursor: grab;
    z-index: 10;
  }}

  #canvas-container:active {{
    cursor: grabbing;
  }}

  canvas {{
    width: 100% !important;
    height: 100% !important;
    display: block;
    outline: none;
  }}

  /* Frosted Glass Prompt Capsule (What you want to ask? ↵) */
  #dock-pill {{
    position: relative;
    width: 440px;
    height: 56px;
    background: linear-gradient(135deg, rgba(246, 238, 230, 0.94) 0%, rgba(234, 222, 212, 0.96) 100%);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1.5px solid rgba(255, 255, 255, 0.7);
    border-radius: 28px;
    box-shadow: 0 14px 34px rgba(74, 62, 61, 0.16), 0 2px 6px rgba(0, 0, 0, 0.04), inset 0 1px 1px rgba(255, 255, 255, 0.9);
    display: flex;
    align-items: center;
    padding: 0 8px 0 22px;
    z-index: 20;
    pointer-events: auto;
    transition: transform 0.2s ease, box-shadow 0.2s ease;
  }}

  #dock-pill:focus-within {{
    box-shadow: 0 16px 40px rgba(255, 171, 118, 0.25), 0 0 0 2.5px rgba(255, 171, 118, 0.5), inset 0 1px 1px rgba(255, 255, 255, 0.9);
    transform: translateY(-2px);
  }}

  #chat-input {{
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: #4A3E3D;
    font-size: 16px;
    font-weight: 500;
    letter-spacing: -0.2px;
  }}

  #chat-input::placeholder {{
    color: rgba(150, 134, 130, 0.75);
    font-weight: 400;
  }}

  #send-btn {{
    width: 40px;
    height: 40px;
    border-radius: 50%;
    border: none;
    background: rgba(255, 255, 255, 0.6);
    color: #796664;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: all 0.2s ease;
    box-shadow: 0 2px 6px rgba(0,0,0,0.06);
  }}

  #send-btn:hover {{
    background: #FFAB76;
    color: #FFFFFF;
    transform: scale(1.05);
    box-shadow: 0 4px 12px rgba(255, 171, 118, 0.35);
  }}

  #send-btn svg {{
    width: 20px;
    height: 20px;
  }}

  /* Dock Toolbar Overlay (Appears on Hover) */
  #dock-toolbar {{
    position: absolute;
    bottom: -32px;
    display: flex;
    gap: 8px;
    opacity: 0;
    transform: translateY(-4px);
    transition: all 0.25s ease;
    pointer-events: auto;
    z-index: 30;
  }}

  #root:hover #dock-toolbar {{
    opacity: 1;
    transform: translateY(0);
  }}

  .tool-chip {{
    background: rgba(255, 252, 248, 0.92);
    border: 1px solid rgba(220, 205, 195, 0.6);
    border-radius: 12px;
    padding: 3px 10px;
    font-size: 11px;
    font-weight: 600;
    color: #796664;
    cursor: pointer;
    box-shadow: 0 2px 6px rgba(0,0,0,0.05);
    transition: all 0.15s ease;
  }}

  .tool-chip:hover {{
    background: #FFAB76;
    color: #FFF;
    border-color: #FFAB76;
  }}
</style>
</head>
<body>

<div id="root">
  <div id="speech-bubble">
    <span id="speech-text">Mırr! Ne sormak istersin? 🐾</span>
  </div>

  <div id="canvas-container"></div>

  <div id="dock-pill">
    <input type="text" id="chat-input" placeholder="What you want to ask?" autocomplete="off" spellcheck="false" />
    <button id="send-btn" title="Send (Enter)">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.3" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="9 10 4 15 9 20"></polyline>
        <path d="M20 4v7a4 4 0 0 1-4 4H4"></path>
      </svg>
    </button>
  </div>

  <div id="dock-toolbar">
    <div class="tool-chip" onclick="window.triggerPetAnim('high_five')">🖐️ Çak!</div>
    <div class="tool-chip" onclick="window.triggerPetAnim('stretch')">🧘 Esne</div>
    <div class="tool-chip" onclick="window.triggerPetAnim('sleep')">💤 Uyu</div>
    <div class="tool-chip" onclick="window.triggerPetAnim('purr')">❤️ Sev</div>
    <div class="tool-chip" onclick="window.toggleArtStyle()">🎨 Piksele Geç</div>
  </div>
</div>

<script>
{three_js}
</script>

<script>
// ---------------------------------------------------------------------------
// Three.js 3D Animated Tuxedo Cat Engine (Oreo)
// ---------------------------------------------------------------------------

let scene, camera, renderer;
let catGroup, headGroup, neckGroup, bodyMesh, tailBones = [], leftArm, rightArm;
let leftEar, rightEar, leftEye, rightEye, mustacheMesh;
let mouse = {{ x: 0, y: 0 }};
let targetHeadRot = {{ x: 0, y: 0 }};
let animTime = 0;
let currentAnim = 'idle';
let animTimer = 0;
let isBlinking = false;
let blinkProgress = 0;

function init3D() {{
  const container = document.getElementById('canvas-container');
  const width = container.clientWidth || 540;
  const height = container.clientHeight || 380;

  scene = new THREE.Scene();

  camera = new THREE.PerspectiveCamera(40, width / height, 0.1, 100);
  camera.position.set(0, 0.8, 4.2);

  renderer = new THREE.WebGLRenderer({{ alpha: true, antialias: true }});
  renderer.setSize(width, height);
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.shadowMap.enabled = true;
  container.appendChild(renderer.domElement);

  // Soft Ambient & Directional Studio Lights
  const ambientLight = new THREE.AmbientLight(0xFFF9F5, 1.1);
  scene.add(ambientLight);

  const keyLight = new THREE.DirectionalLight(0xFFE5D5, 1.2);
  keyLight.position.set(2, 4, 3);
  scene.add(keyLight);

  const fillLight = new THREE.DirectionalLight(0xDCEBFF, 0.6);
  fillLight.position.set(-3, 2, 2);
  scene.add(fillLight);

  build3DCat();
  setupInteractivity();
  animate();
}}

// Build stylized 3D Tuxedo Cat with signature mustache
function build3DCat() {{
  catGroup = new THREE.Group();
  catGroup.position.set(0, -0.65, 0); // Aligns paws resting on top edge of dock pill

  // Materials
  const blackFurMat = new THREE.MeshStandardMaterial({{
    color: 0x1A1818,
    roughness: 0.65,
    metalness: 0.1,
  }});

  const whiteFurMat = new THREE.MeshStandardMaterial({{
    color: 0xFCFAF7,
    roughness: 0.5,
    metalness: 0.05,
  }});

  const pinkInnerMat = new THREE.MeshStandardMaterial({{
    color: 0xFFAFA5,
    roughness: 0.7,
  }});

  const eyeIrisMat = new THREE.MeshStandardMaterial({{
    color: 0xD8A038, // Golden Amber Feline Eyes
    roughness: 0.2,
    metalness: 0.3,
  }});

  const eyePupilMat = new THREE.MeshBasicMaterial({{ color: 0x0A0A0A }});

  // 1. Torso / Body (Black back with white bib/chest)
  const bodyGeo = new THREE.CylinderGeometry(0.55, 0.7, 1.2, 24);
  bodyMesh = new THREE.Mesh(bodyGeo, blackFurMat);
  bodyMesh.position.set(0, 0.6, -0.1);
  catGroup.add(bodyMesh);

  // White chest bib
  const chestGeo = new THREE.SphereGeometry(0.52, 18, 18, 0, Math.PI * 2, 0, Math.PI * 0.5);
  const chestMesh = new THREE.Mesh(chestGeo, whiteFurMat);
  chestMesh.position.set(0, 0.65, 0.12);
  chestMesh.scale.set(0.9, 1.1, 0.75);
  catGroup.add(chestMesh);

  // 2. Neck & Head
  neckGroup = new THREE.Group();
  neckGroup.position.set(0, 1.2, 0);
  catGroup.add(neckGroup);

  headGroup = new THREE.Group();
  neckGroup.add(headGroup);

  // Head sphere
  const headGeo = new THREE.SphereGeometry(0.68, 28, 28);
  const headMesh = new THREE.Mesh(headGeo, blackFurMat);
  headGroup.add(headMesh);

  // White face mask (cheeks and chin)
  const maskGeo = new THREE.SphereGeometry(0.69, 24, 24, 0, Math.PI, 0, Math.PI);
  const maskMesh = new THREE.Mesh(maskGeo, whiteFurMat);
  maskMesh.position.set(0, -0.15, 0.18);
  maskMesh.scale.set(0.72, 0.55, 0.65);
  maskMesh.rotation.x = 0.2;
  headGroup.add(maskMesh);

  // Signature Oreo Mustache (Black patch under the nose)
  const mustacheGeo = new THREE.BoxGeometry(0.32, 0.08, 0.05);
  mustacheMesh = new THREE.Mesh(mustacheGeo, blackFurMat);
  mustacheMesh.position.set(0, -0.12, 0.68);
  mustacheMesh.rotation.y = 0;
  headGroup.add(mustacheMesh);

  // Cute pink nose
  const noseGeo = new THREE.ConeGeometry(0.06, 0.06, 3);
  const noseMesh = new THREE.Mesh(noseGeo, pinkInnerMat);
  noseMesh.rotation.x = Math.PI;
  noseMesh.rotation.z = Math.PI;
  noseMesh.position.set(0, -0.05, 0.70);
  headGroup.add(noseMesh);

  // Left & Right Feline Ears
  const earGeo = new THREE.ConeGeometry(0.24, 0.42, 4);
  earGeo.rotateY(Math.PI / 4);

  leftEar = new THREE.Group();
  leftEar.position.set(-0.42, 0.62, 0.05);
  leftEar.rotation.set(-0.15, 0.2, 0.35);
  const leftEarMesh = new THREE.Mesh(earGeo, blackFurMat);
  leftEar.add(leftEarMesh);

  const innerEarGeo = new THREE.ConeGeometry(0.18, 0.32, 4);
  innerEarGeo.rotateY(Math.PI / 4);
  const leftInner = new THREE.Mesh(innerEarGeo, pinkInnerMat);
  leftInner.position.set(0, -0.02, 0.03);
  leftEar.add(leftInner);
  headGroup.add(leftEar);

  rightEar = new THREE.Group();
  rightEar.position.set(0.42, 0.62, 0.05);
  rightEar.rotation.set(-0.15, -0.2, -0.35);
  const rightEarMesh = new THREE.Mesh(earGeo, blackFurMat);
  rightEar.add(rightEarMesh);

  const rightInner = new THREE.Mesh(innerEarGeo, pinkInnerMat);
  rightInner.position.set(0, -0.02, 0.03);
  rightEar.add(rightInner);
  headGroup.add(rightEar);

  // Expressive Eyes
  const eyeGeo = new THREE.SphereGeometry(0.12, 16, 16);
  const pupilGeo = new THREE.BoxGeometry(0.04, 0.16, 0.04);

  leftEye = new THREE.Group();
  leftEye.position.set(-0.24, 0.08, 0.58);
  const lEyeMesh = new THREE.Mesh(eyeGeo, eyeIrisMat);
  const lPupilMesh = new THREE.Mesh(pupilGeo, eyePupilMat);
  lPupilMesh.position.set(0, 0, 0.10);
  leftEye.add(lEyeMesh);
  leftEye.add(lPupilMesh);
  headGroup.add(leftEye);

  rightEye = new THREE.Group();
  rightEye.position.set(0.24, 0.08, 0.58);
  const rEyeMesh = new THREE.Mesh(eyeGeo, eyeIrisMat);
  const rPupilMesh = new THREE.Mesh(pupilGeo, eyePupilMat);
  rPupilMesh.position.set(0, 0, 0.10);
  rightEye.add(rEyeMesh);
  rightEye.add(rPupilMesh);
  headGroup.add(rightEye);

  // 3. Front Paws Resting on Dock Bar Top Edge
  const pawMat = whiteFurMat;
  const pawGeo = new THREE.SphereGeometry(0.16, 16, 16);
  pawGeo.scale(1.2, 0.7, 1.4);

  leftArm = new THREE.Group();
  leftArm.position.set(-0.45, 0.85, 0.25);
  const lForearm = new THREE.Mesh(new THREE.CylinderGeometry(0.12, 0.14, 0.8), blackFurMat);
  lForearm.position.set(0, -0.4, 0);
  lForearm.rotation.x = 0.35;
  leftArm.add(lForearm);

  const lPaw = new THREE.Mesh(pawGeo, pawMat);
  lPaw.position.set(0, -0.78, 0.24); // Curves directly over the top of the pill
  leftArm.add(lPaw);
  catGroup.add(leftArm);

  rightArm = new THREE.Group();
  rightArm.position.set(0.45, 0.85, 0.25);
  const rForearm = new THREE.Mesh(new THREE.CylinderGeometry(0.12, 0.14, 0.8), blackFurMat);
  rForearm.position.set(0, -0.4, 0);
  rForearm.rotation.x = 0.35;
  rightArm.add(rForearm);

  const rPaw = new THREE.Mesh(pawGeo, pawMat);
  rPaw.position.set(0, -0.78, 0.24); // Curves directly over the top of the pill
  rightArm.add(rPaw);
  catGroup.add(rightArm);

  // 4. Segmented 5-Joint Tail with Smooth Sine Physics
  let prevJoint = catGroup;
  tailBones = [];
  const tailGeo = new THREE.CylinderGeometry(0.08, 0.09, 0.25, 12);

  for (let i = 0; i < 5; i++) {{
    const bone = new THREE.Group();
    if (i === 0) {{
      bone.position.set(0.42, 0.2, -0.45);
      bone.rotation.x = -0.5;
    }} else {{
      bone.position.set(0, 0.22, 0);
    }}
    const tailMesh = new THREE.Mesh(tailGeo, blackFurMat);
    tailMesh.position.set(0, 0.11, 0);
    bone.add(tailMesh);
    prevJoint.add(bone);
    prevJoint = bone;
    tailBones.push(bone);
  }}

  scene.add(catGroup);
}}

function setupInteractivity() {{
  // Mouse cursor tracking for 3D head and eyes
  window.addEventListener('mousemove', (e) => {{
    const nx = (e.clientX / window.innerWidth) * 2 - 1;
    const ny = -(e.clientY / window.innerHeight) * 2 + 1;
    mouse.x = nx;
    mouse.y = ny;
    targetHeadRot.y = nx * 0.45;
    targetHeadRot.x = -ny * 0.35;
  }});

  // Chat input Enter key submission
  const input = document.getElementById('chat-input');
  const sendBtn = document.getElementById('send-btn');

  function submitChat() {{
    const text = input.value.trim();
    if (!text) return;

    window.triggerPetAnim('ask');
    window.postIpcMessage({{ type: 'chat', text: text }});
    input.value = '';
    window.showChatBubble("Düşünüyorum... ✨");
  }}

  sendBtn.addEventListener('click', submitChat);
  input.addEventListener('keydown', (e) => {{
    if (e.key === 'Enter') {{
      submitChat();
    }}
  }});

  // Pet clicking interaction
  const container = document.getElementById('canvas-container');
  container.addEventListener('click', () => {{
    window.triggerPetAnim('purr');
    window.postIpcMessage({{ type: 'interact', action: 'pet' }});
  }});

  // Window dragging support
  const dockPill = document.getElementById('dock-pill');
  dockPill.addEventListener('mousedown', (e) => {{
    if (e.target.id !== 'chat-input' && e.target.id !== 'send-btn' && !e.target.closest('#send-btn')) {{
      window.postIpcMessage({{ type: 'drag' }});
    }}
  }});

  container.addEventListener('mousedown', (e) => {{
    if (e.button === 0) {{
      window.postIpcMessage({{ type: 'drag' }});
    }}
  }});
}}

// 60 FPS Render Loop
function animate() {{
  requestAnimationFrame(animate);
  animTime += 0.035;

  // 1. Thoracic Breathing Cycle
  const breath = Math.sin(animTime * 2.2);
  bodyMesh.scale.set(1 + breath * 0.02, 1 + breath * 0.015, 1 + breath * 0.03);
  neckGroup.position.y = 1.2 + breath * 0.015;

  // 2. Smooth Head & Eye Look-At (Damped Lerp)
  headGroup.rotation.y += (targetHeadRot.y - headGroup.rotation.y) * 0.08;
  headGroup.rotation.x += (targetHeadRot.x - headGroup.rotation.x) * 0.08;

  // 3. Multi-Joint Tail Physics
  for (let i = 0; i < tailBones.length; i++) {{
    tailBones[i].rotation.z = Math.sin(animTime * 2.5 + i * 0.5) * 0.16;
    tailBones[i].rotation.x = -0.15 + Math.cos(animTime * 1.5 + i * 0.4) * 0.08;
  }}

  // 4. Autonomic Blinking
  if (Math.random() < 0.008 && !isBlinking) {{
    isBlinking = true;
    blinkProgress = 0;
  }}

  if (isBlinking) {{
    blinkProgress += 0.18;
    const eyeScaleY = Math.abs(Math.sin(blinkProgress * Math.PI));
    leftEye.scale.y = Math.max(0.1, 1 - eyeScaleY);
    rightEye.scale.y = Math.max(0.1, 1 - eyeScaleY);
    if (blinkProgress >= 1.0) {{
      isBlinking = false;
      leftEye.scale.y = 1;
      rightEye.scale.y = 1;
    }}
  }}

  // 5. Special Animation States
  if (currentAnim === 'high_five') {{
    animTimer -= 0.04;
    rightArm.position.y = 0.85 + Math.sin(animTimer * Math.PI) * 0.35;
    rightArm.rotation.x = -Math.sin(animTimer * Math.PI) * 0.6;
    if (animTimer <= 0) {{
      currentAnim = 'idle';
      rightArm.position.set(0.45, 0.85, 0.25);
      rightArm.rotation.set(0, 0, 0);
    }}
  }} else if (currentAnim === 'stretch') {{
    animTimer -= 0.03;
    bodyMesh.position.y = 0.6 + Math.sin(animTimer * Math.PI) * 0.2;
    headGroup.rotation.x = -Math.sin(animTimer * Math.PI) * 0.4;
    if (animTimer <= 0) {{
      currentAnim = 'idle';
      bodyMesh.position.set(0, 0.6, -0.1);
    }}
  }} else if (currentAnim === 'purr') {{
    animTimer -= 0.05;
    catGroup.position.y = -0.65 + Math.sin(animTimer * 16) * 0.03;
    if (animTimer <= 0) {{
      currentAnim = 'idle';
      catGroup.position.y = -0.65;
    }}
  }} else if (currentAnim === 'sleep') {{
    headGroup.rotation.x = 0.35;
    headGroup.position.y = -0.15;
    leftEye.scale.y = 0.1;
    rightEye.scale.y = 0.1;
  }}

  renderer.render(scene, camera);
}}

// Public Controls called from Rust
window.triggerPetAnim = function(name) {{
  currentAnim = name;
  animTimer = 1.0;
  if (name === 'high_five') {{
    window.showChatBubble("Çak bir beşlik! 🐾🖐️");
  }} else if (name === 'purr') {{
    window.showChatBubble("Mırrr... Çok sevildim! ❤️");
  }} else if (name === 'stretch') {{
    window.showChatBubble("Ooh, güzelce esnedim! ✨");
  }} else if (name === 'sleep') {{
    window.showChatBubble("Mırr... Biraz kestireyim... zZz 💤");
  }}
}};

window.showChatBubble = function(text) {{
  const bubble = document.getElementById('speech-bubble');
  const span = document.getElementById('speech-text');
  span.innerText = text;
  bubble.classList.add('visible');

  clearTimeout(window.bubbleTimeout);
  window.bubbleTimeout = setTimeout(() => {{
    bubble.classList.remove('visible');
  }}, 6500);
}};

window.setCompanionStyle = function(style) {{
  if (style === 'pixel') {{
    window.showChatBubble("Piksel Sanatı moduna geçiliyor! 🎨");
  }} else {{
    window.showChatBubble("3D Gerçekçi Kedi (Oreo) aktif! 📸");
  }}
}};

window.toggleArtStyle = function() {{
  window.postIpcMessage({{ type: 'toggle_style' }});
}};

window.postIpcMessage = function(data) {{
  if (window.ipc) {{
    window.ipc.postMessage(JSON.stringify(data));
  }} else if (window.chrome && window.chrome.webview) {{
    window.chrome.webview.postMessage(JSON.stringify(data));
  }}
}};

// Initialize after DOM loads
window.addEventListener('DOMContentLoaded', init3D);
</script>
</body>
</html>
"#,
        three_js = THREE_JS_LIB
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dock_command_variants() {
        let cmd = Dock3DCommand::ChatMessage {
            sender: "User".into(),
            text: "Hello cat".into(),
        };
        match cmd {
            Dock3DCommand::ChatMessage { text, .. } => assert_eq!(text, "Hello cat"),
            _ => panic!("Variant mismatch"),
        }
    }

    #[test]
    fn test_dock_html_generation_includes_three_and_pill() {
        let html = generate_dock_html();
        assert!(html.contains("What you want to ask?"));
        assert!(html.contains("speech-bubble"));
        assert!(html.contains("THREE.PerspectiveCamera"));
        assert!(html.contains("mustacheMesh"));
    }
}
