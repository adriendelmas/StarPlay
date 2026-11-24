// Slint interface: transparent window placed over the video (player mode, #3).

slint::slint! {
    #[edition = "1.19"]

    export struct Track { id: int, label: string, selected: bool }
    export struct Recent { path: string, name: string, folder: string }
    export struct StripItem { frame: int, image: image, ready: bool, current: bool }

    // Icons drawn as vector paths (24×24 grid)
    global Icons {
        out property <string> play: "M 7 4.5 L 7 19.5 Q 7 20.6 8 20 L 19.5 12.9 Q 20.4 12.3 19.5 11.7 L 8 4 Q 7 3.4 7 4.5 Z";
        out property <string> pause: "M 6 4 L 10 4 L 10 20 L 6 20 Z M 14 4 L 18 4 L 18 20 L 14 20 Z";
        out property <string> back: "M 11 6 L 11 18 L 2.5 12 Z M 21 6 L 21 18 L 12.5 12 Z";
        out property <string> forward: "M 3 6 L 3 18 L 11.5 12 Z M 13 6 L 13 18 L 21.5 12 Z";
        out property <string> speaker: "M 3 9 L 7 9 L 12 4.5 L 12 19.5 L 7 15 L 3 15 Z";
        out property <string> waves: "M 15.5 9 Q 17.5 12 15.5 15 M 18 6.5 Q 22 12 18 17.5";
        out property <string> muted: "M 16 9 L 21.5 15 M 21.5 9 L 16 15";
        out property <string> fullscreen: "M 4 9 L 4 4 L 9 4 M 15 4 L 20 4 L 20 9 M 20 15 L 20 20 L 15 20 M 9 20 L 4 20 L 4 15";
        out property <string> menu: "M 4 6 L 20 6 M 4 12 L 20 12 M 4 18 L 14 18";
        // subtitles / audio: frame with two lines of text; speed: dial with a needle
        out property <string> subtitles: "M 4.5 5.5 L 19.5 5.5 Q 21 5.5 21 7 L 21 17 Q 21 18.5 19.5 18.5 L 4.5 18.5 Q 3 18.5 3 17 L 3 7 Q 3 5.5 4.5 5.5 Z M 6.5 11 L 10.5 11 M 13 11 L 17.5 11 M 6.5 14.5 L 15 14.5";
        out property <string> speedometer: "M 4.2 17.5 A 8.5 8.5 0 1 1 19.8 17.5 M 12 13.5 L 16 8.5 M 12 5 L 12 6.5 M 5.5 11 L 7 11 M 17 11 L 18.5 11";
        out property <string> step-back: "M 5.5 5 L 7.5 5 L 7.5 19 L 5.5 19 Z M 19 5.5 L 19 18.5 L 9.5 12 Z";
        out property <string> step-forward: "M 16.5 5 L 18.5 5 L 18.5 19 L 16.5 19 Z M 5 5.5 L 5 18.5 L 14.5 12 Z";
        // Mode toggle: screen with play (player), viewfinder with a measurement curve (verify)
        out property <string> mode-screen: "M 4.5 5.5 L 19.5 5.5 Q 21 5.5 21 7 L 21 17 Q 21 18.5 19.5 18.5 L 4.5 18.5 Q 3 18.5 3 17 L 3 7 Q 3 5.5 4.5 5.5 Z";
        out property <string> mode-play: "M 10 9 L 10 15 L 15 12 Z";
        out property <string> mode-analysis: "M 3 8 L 3 4.5 Q 3 3 4.5 3 L 8 3 M 16 3 L 19.5 3 Q 21 3 21 4.5 L 21 8 M 21 16 L 21 19.5 Q 21 21 19.5 21 L 16 21 M 8 21 L 4.5 21 Q 3 21 3 19.5 L 3 16 M 6.5 15.5 L 9 11.5 L 11.5 13.5 L 14.5 8 L 17.5 12.5";
        out property <string> minimize: "M 6.5 12 L 17.5 12";
        out property <string> maximize: "M 7 7 L 17 7 L 17 17 L 7 17 Z";
        out property <string> restore: "M 7 9.5 L 14.5 9.5 L 14.5 17 L 7 17 Z M 9.5 7 L 17 7 L 17 14.5";
        out property <string> close: "M 7.5 7.5 L 16.5 16.5 M 16.5 7.5 L 7.5 16.5";
    }

    component IconButton inherits Rectangle {
        in property <string> fill-icon;
        in property <string> stroke-icon;
        in property <length> icon-size: 22px;
        in property <color> hover-color: #ffffff1a;
        in property <bool> selected;
        callback clicked;
        width: 34px;
        height: 34px;
        border-radius: 8px;
        background: root.selected ? #ffffff38 : ta.pressed ? #ffffff30 : ta.has-hover ? root.hover-color : transparent;
        animate background { duration: 120ms; }
        if root.fill-icon != "" : Path {
            width: root.icon-size; height: root.icon-size;
            viewbox-width: 24; viewbox-height: 24;
            commands: root.fill-icon;
            fill: white;
        }
        if root.stroke-icon != "" : Path {
            width: root.icon-size; height: root.icon-size;
            viewbox-width: 24; viewbox-height: 24;
            commands: root.stroke-icon;
            stroke: white;
            stroke-width: max(1.2px, 1.8px * root.icon-size / 22px); // proportional stroke: thin on small icons
            stroke-line-cap: round;
        }
        ta := TouchArea { clicked => { root.clicked(); } }
    }

    // Clickable and draggable bar (progress, volume). scrub(value 0..1, released?)
    component Scrubber inherits Rectangle {
        in property <float> value;
        // Optional tick marks (positions 0..1): one per frame when the timeline is zoomed
        in property <int> ticks;
        in property <float> tick-start;
        in property <float> tick-step;
        callback scrub(float, bool);
        callback wheel(float, bool);   // wheel notch (> 0 upward), Ctrl held?
        out property <length> mouse-x: ta.mouse-x;
        out property <length> mouse-y: ta.mouse-y;
        out property <length> press-y;
        in property <bool> hold;       // the handle follows the value, not the mouse (fine adjustment)
        in property <MouseCursor> cursor: MouseCursor.default;
        out property <bool> hovered: ta.has-hover || ta.pressed;
        out property <length> hover-x: clamp(ta.mouse-x, 0, self.width);
        property <float> drag-value;
        property <float> shown: ta.pressed && !root.hold ? self.drag-value : clamp(self.value, 0, 1);
        height: 18px;
        Rectangle {
            y: (parent.height - self.height) / 2;
            height: root.hovered ? 6px : 4px;
            animate height { duration: 120ms; }
            border-radius: self.height / 2;
            background: #ffffff38;
            Rectangle {
                x: 0;
                width: parent.width * root.shown;
                border-radius: parent.border-radius;
                background: #ffffffeb;
            }
        }
        for i in root.ticks : Rectangle {
            x: parent.width * (root.tick-start + i * root.tick-step);
            y: parent.height - 3px;
            width: 1px;
            height: 3px;
            background: #ffffff80;
        }
        if root.hovered : Rectangle {
            x: parent.width * root.shown - 6px;
            y: (parent.height - 12px) / 2;
            width: 12px; height: 12px;
            border-radius: 6px;
            background: white;
            drop-shadow-blur: 4px;
            drop-shadow-color: #00000080;
        }
        ta := TouchArea {
            mouse-cursor: root.cursor;
            pointer-event(e) => {
                if (e.button == PointerEventButton.left) {
                    if (e.kind == PointerEventKind.down) { root.press-y = self.mouse-y; }
                    root.drag-value = clamp(self.mouse-x / self.width, 0, 1);
                    root.scrub(root.drag-value, e.kind == PointerEventKind.up);
                }
            }
            moved => {
                if (self.pressed) {
                    root.drag-value = clamp(self.mouse-x / self.width, 0, 1);
                    root.scrub(root.drag-value, false);
                }
            }
            scroll-event(e) => {
                if (e.delta-y != 0) { root.wheel(e.delta-y / 1px, e.modifiers.control); }
                accept
            }
        }
    }

    // Window edge: pressing starts the native resize (Windows HT* code)
    component Edge inherits TouchArea {
        in property <int> hit;
        callback drag(int);
        pointer-event(e) => {
            if (e.kind == PointerEventKind.down && e.button == PointerEventButton.left) { root.drag(root.hit); }
        }
    }

    // Glass drawn by the interface where mpv does not reach (black bars, home): glass on black,
    // it is the rim, lighter at the top left like the one in the shader.
    component GlassRing inherits Rectangle {
        border-radius: min(self.width, self.height) / 2;
        background: #ffffff08;
        border-width: 1px;
        border-color: @linear-gradient(135deg, #ffffffb0 0%, #ffffff30 60%, #ffffff60 100%);
    }

    // Value to drag, Houdini-parameter style: a horizontal click-drag changes the value; a scale
    // (1 / 0.1 / 0.01) shows above it and moving the mouse up picks a larger step. Single click = keyboard input,
    // double-click = default value.
    component DragValue inherits Rectangle {
        in property <float> value;
        in property <string> text;
        in property <bool> editing;
        in property <float> minimum: 0;
        in property <float> maximum: 100;
        callback edited(float);
        callback clicked;
        callback reset;
        property <length> last-x;
        property <length> press-y;
        property <bool> dragging;
        property <float> acc;
        property <int> level: ta.pressed && root.dragging ? clamp(floor((root.press-y - ta.mouse-y) / 24px), 0, 2) : 0;
        property <float> step: root.level == 2 ? 1 : root.level == 1 ? 0.1 : 0.01;
        width: 90px;
        height: 26px;
        border-radius: 6px;
        background: root.editing ? #ffffff38 : ta.pressed ? #ffffff30 : ta.has-hover ? #ffffff1f : #ffffff12;
        Text { text: root.text; color: white; font-size: 12px; font-family: "Consolas"; }
        if ta.pressed && root.dragging : Rectangle {
            x: (parent.width - self.width) / 2;
            y: -self.height - 6px;
            width: 56px;
            height: 3 * 22px + 8px;
            border-radius: 8px;
            background: #000000c8;
            VerticalLayout {
                padding: 4px;
                for label[i] in ["1", "0.1", "0.01"] : Rectangle {
                    height: 22px;
                    border-radius: 5px;
                    background: 2 - i == root.level ? #ffffff38 : transparent;
                    Text { text: label; color: white; font-size: 11px; font-family: "Consolas"; }
                }
            }
        }
        ta := TouchArea {
            mouse-cursor: MouseCursor.ew-resize;
            pointer-event(e) => {
                if (e.kind == PointerEventKind.down && e.button == PointerEventButton.left) {
                    root.last-x = self.mouse-x;
                    root.press-y = self.mouse-y;
                    root.dragging = false;
                    root.acc = root.value;
                }
            }
            moved => {
                if (self.pressed) {
                    if (!root.dragging && abs(self.mouse-x - root.last-x) > 3px) { root.dragging = true; }
                    if (root.dragging) {
                        // 8 px = one step; the step depends on the mouse height (scale)
                        root.acc = clamp(root.acc + (self.mouse-x - root.last-x) / 8px * root.step, root.minimum, root.maximum);
                        root.last-x = self.mouse-x;
                        root.edited(round(root.acc * 100) / 100);
                    }
                }
            }
            clicked => { if (!root.dragging) { root.clicked(); } }
            double-clicked => { root.reset(); }
        }
    }

    component MenuItem inherits Rectangle {
        in property <string> text;
        in property <bool> checked;
        callback clicked;
        height: 30px;
        border-radius: 6px;
        background: ta.has-hover ? #ffffff1f : transparent;
        HorizontalLayout {
            padding-left: 8px; padding-right: 10px; spacing: 6px;
            Text { width: 14px; text: root.checked ? "✓" : ""; color: white; vertical-alignment: center; }
            Text { text: root.text; color: white; font-size: 13px; vertical-alignment: center; overflow: elide; }
        }
        ta := TouchArea { clicked => { root.clicked(); } }
    }

    component MenuTitle inherits Text {
        color: #ffffff80;
        font-size: 11px;
        font-weight: 600;
    }

    export component Overlay inherits Window {
        no-frame: true;
        background: transparent;
        title: "StarPlay";
        default-font-size: 13px;

        in property <bool> has-video;
        in property <bool> paused;
        in property <float> position;   // seconds
        in property <float> duration;
        in property <float> volume;     // 0..100
        in property <bool> muted;
        in property <float> speed: 1;
        in property <[Track]> audio-tracks;
        in property <[Track]> sub-tracks;
        in property <[Recent]> recent;
        in property <string> stats;
        in property <bool> show-stats;
        in property <bool> is-maximized;
        in property <bool> is-fullscreen;
        // Black bars around the video: glass exists only on the image, the controls stay inside
        in property <length> video-left;
        in property <length> video-top;
        in property <length> video-right;
        in property <length> video-bottom;
        // Verify mode (#6) and precise timeline (#7)
        in-out property <bool> verify;
        in property <float> fps: 25;
        in property <int> frame;           // displayed frame (numbered from 0)
        in property <int> total-frames;
        in property <image> preview;       // preview of the hovered frame (preview engine)
        // Active input field: 0 none, 1 "go to", 2 speed (keys forwarded by the host window)
        in property <int> edit-field;
        in property <string> edit-text;
        // Precision drag (YouTube style): moving the mouse up during a drag on the timeline
        in-out property <bool> precise;
        // 0..1: how far the mouse was lifted during the drag; the panel grows upward by as much
        property <float> lift;
        animate lift { duration: 180ms; easing: ease-out; }
        property <float> lift-soft: root.lift * root.lift * (3 - 2 * root.lift);   // smooth-curve fade
        // Verify mode: the bar is a full-width sheet (edges and bottom off screen); animated on mode change
        property <float> wide: root.verify ? 1 : 0;
        animate wide { duration: 220ms; easing: ease-in-out; }
        // Fine-adjust row: the frames grow as the mouse rises, and the mouse travels as many pixels per
        // frame as the frame is wide (64 px near the bar: fast; ~240 px at the very top: very precise)
        property <length> pitch-max: min(248px, (root.width - 32px) / 3);
        property <length> pitch: 64px + (root.pitch-max - 64px) * root.lift;
        property <length> thumb-w: root.pitch - 8px;
        property <length> strip-h: 14px + 22px + root.thumb-w * 9 / 16 + 30px;
        out property <int> strip-radius: min(10, ceil(root.width / 2 / root.pitch) + 1);
        property <float> fine-pos;          // targeted frame index, fractional: the row scrolls continuously
        in-out property <length> last-x;    // corrected by Rust when it recenters the mouse (endless scrolling)
        in property <[StripItem]> strip;
        in property <int> strip-center;
        in-out property <bool> zoomed;
        in-out property <float> view-start;
        in-out property <float> view-end;
        property <float> v0: root.zoomed ? root.view-start : 0;
        property <float> v1: root.zoomed ? root.view-end : root.duration;
        property <int> fps-int: max(1, round(root.fps));
        property <float> hover-t: root.v0 + progress.hover-x / max(1px, progress.width) * (root.v1 - root.v0);
        property <int> hover-frame: min(max(0, root.total-frames - 1), round(root.hover-t * root.fps));
        in-out property <bool> controls-visible: true;
        in-out property <int> menu-kind;      // open menu: 0 none, 1 subtitles / audio, 2 speed
        property <bool> menu-open: root.menu-kind != 0;
        out property <bool> panel-hovered: panel-ta.has-hover || root.menu-open;
        // Glass geometry (logical pixels), drawn by mpv under the interface (shaders/glass.glsl)
        out property <float> panel-px: sheet.x / 1px;
        out property <float> panel-py: sheet.y / 1px;
        out property <float> panel-pw: sheet.width / 1px;
        out property <float> panel-ph: sheet.height / 1px;
        out property <float> panel-alpha: panel.visible ? panel.opacity : 0;
        out property <float> menu-px: menu.x / 1px;
        out property <float> menu-py: menu.y / 1px;
        out property <float> menu-pw: menu.width / 1px;
        out property <float> menu-ph: menu.height / 1px;
        out property <float> menu-alpha: menu.visible ? panel.opacity : 0;
        out property <float> top-px: top.x / 1px;
        out property <float> top-py: top.y / 1px;
        out property <float> win-d: 24;      // diameter of a window button circle
        out property <float> win-step: 32;   // from one circle to the next (diameter + gap)
        out property <float> top-alpha: root.has-video ? top.opacity : 0;
        out property <float> mode-px: mode.x / 1px;
        out property <float> mode-py: mode.y / 1px;
        out property <float> mode-pw: mode.width / 1px;
        out property <float> mode-ph: mode.height / 1px;
        out property <float> mode-alpha: mode.visible ? mode.opacity : 0;
        out property <float> radius-panel: 22;
        out property <float> radius-menu: 14;
        property <bool> resizable: !root.is-maximized && !root.is-fullscreen;
        property <length> press-x;
        property <length> press-y;
        property <bool> dragged;

        callback activity();
        callback toggle-pause();
        callback toggle-fullscreen();
        callback seek(float, bool);        // seconds, exact?
        callback skip(float);              // relative seconds
        callback set-volume(float);
        callback volume-step(float);
        callback toggle-mute();
        callback set-speed(float);
        callback select-audio(int);
        callback select-sub(int);
        callback open-dialog();
        callback open-path(string);
        callback window-drag(int);         // HT* code: 2 = move, 10..17 = edges
        callback minimize();
        callback maximize();
        callback close-window();
        callback hover-time(float, int);   // timeline hover: time, frame -> preview
        callback step-frames(int);         // one frame forward (+1) / backward (-1)
        callback begin-edit(int);          // opens an input field (1 go to, 2 speed)
        callback precise-frame(int, bool); // fine adjust: targeted frame (index), released?
        callback precise-start();          // start of fine adjust (pauses playback)

        pure function two(n: float) -> string { return (n < 10 ? "0" : "") + n; }
        pure function fmt(t: float) -> string {
            if (t >= 3600) {
                return floor(t / 3600) + ":" + two(floor((t - floor(t / 3600) * 3600) / 60)) + ":" + two(floor(t - floor(t / 60) * 60));
            }
            return floor(t / 60) + ":" + two(floor(t - floor(t / 60) * 60));
        }

        // Timecode HH:MM:SS:FF from a frame number (nominal rate rounded, no drop-frame)
        pure function tc(n: int) -> string {
            if (n < 0) { return "--:--:--:--"; }
            return two(floor(n / (3600 * root.fps-int))) + ":" + two(mod(floor(n / (60 * root.fps-int)), 60)) + ":"
                + two(mod(floor(n / root.fps-int), 60)) + ":" + two(mod(n, root.fps-int));
        }
        // Timeline zoom around the targeted time (k < 1: zooms in)
        function zoom-at(t: float, ratio: float, k: float) {
            if (root.duration <= 0) { return; }
            let span = clamp((root.v1 - root.v0) * k, 20 / max(1, root.fps), root.duration);
            if (span >= root.duration) { root.zoomed = false; return; }
            let start = clamp(t - ratio * span, 0, root.duration - span);
            root.view-start = start;
            root.view-end = start + span;
            root.zoomed = true;
        }
        // During playback, the zoomed view follows the playhead
        changed position => {
            if (root.zoomed && (root.position > root.view-end || root.position < root.view-start)) {
                let span = root.view-end - root.view-start;
                let start = clamp(root.position - 0.1 * span, 0, max(0, root.duration - span));
                root.view-start = start;
                root.view-end = start + span;
            }
        }
        changed hover-t => {
            if (root.verify && progress.hovered && root.duration > 0 && !root.precise) { root.hover-time(root.hover-frame / root.fps, root.hover-frame); }
        }

        // Background: click = pause, double-click = fullscreen, wheel = volume, movement = show controls
        bg := TouchArea {
            mouse-cursor: root.controls-visible || root.verify || !root.has-video ? MouseCursor.default : MouseCursor.none;
            changed mouse-x => { root.activity(); }
            changed mouse-y => { root.activity(); }
            pointer-event(e) => {
                if (e.kind == PointerEventKind.down) {
                    root.press-x = self.mouse-x;
                    root.press-y = self.mouse-y;
                    root.dragged = false;
                }
            }
            // Dragging the image moves the window; a click without dragging stays play/pause
            moved => {
                if (self.pressed && !root.dragged && !root.is-fullscreen
                    && (abs(self.mouse-x - root.press-x) > 4px || abs(self.mouse-y - root.press-y) > 4px)) {
                    root.dragged = true;
                    root.window-drag(2);
                }
            }
            clicked => {
                if (root.dragged) { }
                else if (root.menu-open) { root.menu-kind = 0; }
                else if (root.has-video) { root.toggle-pause(); }
            }
            double-clicked => { if (root.has-video) { root.toggle-fullscreen(); } }
            scroll-event(e) => {
                if (root.has-video && e.delta-y != 0) {
                    root.volume-step(e.delta-y > 0 ? 5 : -5);
                    root.activity();
                }
                accept
            }
        }

        if root.show-stats : Rectangle {
            x: 12px; y: 12px;
            width: stats-text.preferred-width + 20px;
            height: stats-text.preferred-height + 14px;
            background: #000000b0;
            border-radius: 8px;
            stats-text := Text { text: root.stats; color: white; font-size: 12px; font-family: "Consolas"; }
        }

        // Home screen without video
        if !root.has-video : Rectangle {
            background: #161618;
            border-radius: root.resizable ? 8px : 0;
            VerticalLayout {
                        width: min(440px, root.width - 48px);
                        height: self.preferred-height;
                        x: (parent.width - self.width) / 2;
                        y: max(24px, (parent.height - self.height) / 2);
                        spacing: 10px;
                        Text { text: "StarPlay"; color: white; font-size: 30px; font-weight: 600; horizontal-alignment: center; }
                        Text { text: @tr("Drop a video anywhere in the window"); color: #ffffff8c; font-size: 14px; horizontal-alignment: center; }
                        HorizontalLayout {
                            alignment: center;
                            padding-top: 10px;
                            padding-bottom: 18px;
                            Rectangle {
                                width: 170px; height: 34px;
                                border-radius: 8px;
                                background: open-ta.pressed ? #ffffff38 : open-ta.has-hover ? #ffffff2a : #ffffff1c;
                                Text { text: @tr("Open File…"); color: white; font-size: 13px; }
                                open-ta := TouchArea { clicked => { root.open-dialog(); } }
                            }
                        }
                        if root.recent.length > 0 : MenuTitle { text: @tr("RECENT"); }
                        for r in root.recent : Rectangle {
                            height: 46px;
                            border-radius: 8px;
                            background: r-ta.has-hover ? #ffffff14 : transparent;
                            VerticalLayout {
                                padding-left: 10px; padding-right: 10px;
                                alignment: center;
                                Text { text: r.name; color: white; font-size: 13px; overflow: elide; }
                                Text { text: r.folder; color: #ffffff66; font-size: 11px; overflow: elide; }
                            }
                            r-ta := TouchArea { clicked => { root.open-path(r.path); } }
                        }
            }
        }

        // Floating control panel
        panel := Rectangle {
            width: root.verify ? root.width - 32px : min(620px, root.width - 32px); // verify: full width
            // verify mode: full-width bar stuck to the bottom, without the floating panel margin
            height: root.verify ? 80px : 100px;
            x: (root.width - self.width) / 2;
            y: root.height - root.video-bottom - self.height - (root.verify ? 6px : 22px);
            visible: root.has-video && self.opacity > 0;
            opacity: root.controls-visible || root.verify ? 1 : 0;
            animate opacity { duration: 220ms; easing: ease-in-out; }
            panel-ta := TouchArea {
                changed mouse-x => { root.activity(); }
                VerticalLayout {
                    padding: 12px;
                    padding-left: 18px;
                    padding-right: 18px;
                    HorizontalLayout {
                        spacing: 12px;
                        Text {
                            width: root.verify ? 82px : 58px;
                            text: root.verify ? tc(root.frame) : fmt(root.position);
                            color: #ffffffd0; font-size: 12px; horizontal-alignment: right; vertical-alignment: center;
                            font-family: root.verify ? "Consolas" : "";
                        }
                        progress := Scrubber {
                            value: root.v1 > root.v0 ? (root.position - root.v0) / (root.v1 - root.v0) : 0;
                            hold: root.precise;
                            cursor: root.precise ? MouseCursor.none : MouseCursor.default;
                            scrub(v, done) => {
                                // Verify mode: as soon as the mouse rises above the bar, fine adjust; the row
                                // of frames scrolls with the mouse, all the more finely the higher it is raised
                                root.lift = root.verify && !done ? clamp((self.press-y - self.mouse-y - 12px) / 170px, 0, 1) : 0;
                                if (root.verify && !root.precise && root.lift > 0) {
                                    root.precise = true;
                                    root.fine-pos = (root.v0 + v * (root.v1 - root.v0)) * root.fps;
                                    root.last-x = self.mouse-x;
                                    root.precise-start();
                                }
                                if (root.precise) {
                                    root.fine-pos = clamp(root.fine-pos + (self.mouse-x - root.last-x) / root.pitch, 0, max(0, root.total-frames - 1));
                                    root.last-x = self.mouse-x;
                                    root.precise-frame(round(root.fine-pos), done);
                                    if (done) { root.precise = false; }
                                } else {
                                    root.seek(root.v0 + v * (root.v1 - root.v0), done);
                                }
                            }
                            // Wheel: one frame; Ctrl + wheel: zoom around the mouse (verify mode)
                            wheel(d, ctrl) => {
                                if (!root.verify) { root.skip(d > 0 ? 5 : -5); }
                                else if (ctrl) { root.zoom-at(root.hover-t, self.hover-x / max(1px, self.width), d > 0 ? 0.7 : 1.4); }
                                else { root.step-frames(d > 0 ? 1 : -1); }
                            }
                            // One tick mark per frame when they are at least 4 px apart (zoomed timeline)
                            property <float> frames-in-view: (root.v1 - root.v0) * root.fps;
                            ticks: root.verify && self.frames-in-view * 4px < self.width ? ceil(self.frames-in-view) + 1 : 0;
                            tick-start: ((ceil(root.v0 * root.fps) / root.fps) - root.v0) / max(0.000001, root.v1 - root.v0);
                            tick-step: 1 / max(1, self.frames-in-view);
                        }
                        Text {
                            width: root.verify ? 82px : 58px;
                            text: root.verify ? tc(root.total-frames - 1) : fmt(root.duration);
                            color: #ffffffd0; font-size: 12px; vertical-alignment: center;
                            font-family: root.verify ? "Consolas" : "";
                        }
                    }
                    Rectangle { height: 8px; }
                    if root.verify : HorizontalLayout {
                        // left: frame number, clickable to type a frame or a timecode
                        HorizontalLayout {
                            width: 230px;
                            alignment: start;
                            Rectangle {
                                width: frame-text.preferred-width + 20px;
                                height: 30px;
                                y: 2px;
                                border-radius: 8px;
                                background: root.edit-field == 1 ? #ffffff38 : frame-ta.has-hover ? #ffffff1a : transparent;
                                frame-text := Text {
                                    text: root.edit-field == 1 ? @tr("Go to: {}", root.edit-text) + "▏"
                                        : @tr("Frame {} / {}", root.frame + 1, root.total-frames);
                                    color: white; font-size: 13px; font-family: "Consolas";
                                }
                                frame-ta := TouchArea { clicked => { root.begin-edit(1); } }
                            }
                        }
                        // center: previous frame, play, next frame
                        HorizontalLayout {
                            alignment: center;
                            spacing: 22px;
                            IconButton { fill-icon: Icons.step-back; clicked => { root.step-frames(-1); } }
                            IconButton {
                                width: 42px; height: 42px; y: -4px;
                                icon-size: 30px;
                                fill-icon: root.paused ? Icons.play : Icons.pause;
                                clicked => { root.toggle-pause(); }
                            }
                            IconButton { fill-icon: Icons.step-forward; clicked => { root.step-frames(1); } }
                        }
                        // right: frame rate, menu, fullscreen
                        HorizontalLayout {
                            width: 230px;
                            spacing: 6px;
                            alignment: end;
                            Text { text: @tr("{} fps", round(root.fps * 1000) / 1000); color: #ffffff99; font-size: 12px; vertical-alignment: center; }
                            IconButton {
                                stroke-icon: Icons.subtitles; selected: root.menu-kind == 1;
                                clicked => { root.menu-kind = root.menu-kind == 1 ? 0 : 1; }
                            }
                            IconButton {
                                stroke-icon: Icons.speedometer; selected: root.menu-kind == 2;
                                clicked => { root.menu-kind = root.menu-kind == 2 ? 0 : 2; }
                            }
                            IconButton { stroke-icon: Icons.fullscreen; clicked => { root.toggle-fullscreen(); } }
                        }
                    }
                    if !root.verify : HorizontalLayout {
                        // left: volume
                        HorizontalLayout {
                            width: 170px;
                            spacing: 6px;
                            alignment: start;
                            IconButton {
                                fill-icon: Icons.speaker;
                                stroke-icon: root.muted || root.volume == 0 ? Icons.muted : Icons.waves;
                                clicked => { root.toggle-mute(); }
                            }
                            Scrubber {
                                width: 92px;
                                y: (34px - self.height) / 2;
                                value: root.muted ? 0 : root.volume / 100;
                                scrub(v, done) => { root.set-volume(v * 100); }
                            }
                        }
                        // center: transport
                        HorizontalLayout {
                            alignment: center;
                            spacing: 22px;
                            IconButton { fill-icon: Icons.back; clicked => { root.skip(-10); } }
                            IconButton {
                                width: 42px; height: 42px; y: -4px;
                                icon-size: 30px;
                                fill-icon: root.paused ? Icons.play : Icons.pause;
                                clicked => { root.toggle-pause(); }
                            }
                            IconButton { fill-icon: Icons.forward; clicked => { root.skip(10); } }
                        }
                        // right: speed, menu, fullscreen
                        HorizontalLayout {
                            width: 170px;
                            spacing: 6px;
                            alignment: end;
                            if root.speed != 1 : Rectangle {
                                width: speed-text.preferred-width + 16px;
                                height: 22px;
                                y: 6px;
                                border-radius: 11px;
                                background: speed-ta.has-hover ? #ffffff40 : #ffffff26;
                                speed-text := Text { text: root.speed + "×"; color: white; font-size: 12px; font-weight: 600; }
                                speed-ta := TouchArea { clicked => { root.set-speed(1); } }
                            }
                            IconButton {
                                stroke-icon: Icons.subtitles; selected: root.menu-kind == 1;
                                clicked => { root.menu-kind = root.menu-kind == 1 ? 0 : 1; }
                            }
                            IconButton {
                                stroke-icon: Icons.speedometer; selected: root.menu-kind == 2;
                                clicked => { root.menu-kind = root.menu-kind == 2 ? 0 : 2; }
                            }
                            IconButton { stroke-icon: Icons.fullscreen; clicked => { root.toggle-fullscreen(); } }
                        }
                    }
                }
            }
        }

        // Player / verify toggle (#6), top left; same mpv engine, nothing is reloaded
        mode := Rectangle {
            x: 14px; y: 14px;
            width: mode-layout.preferred-width;
            height: 28px;
            opacity: root.has-video && (root.controls-visible || root.verify) ? 1 : 0;
            visible: self.opacity > 0;
            animate opacity { duration: 220ms; easing: ease-in-out; }
            TouchArea { changed mouse-x => { root.activity(); } }
            mode-layout := HorizontalLayout {
                padding: 3px;
                spacing: 2px;
                IconButton {
                    width: 36px; height: 22px; border-radius: 11px;
                    icon-size: 16px; stroke-icon: Icons.mode-screen; fill-icon: Icons.mode-play;
                    selected: !root.verify;
                    clicked => { root.verify = false; root.activity(); }
                }
                IconButton {
                    width: 36px; height: 22px; border-radius: 11px;
                    icon-size: 16px; stroke-icon: Icons.mode-analysis;
                    selected: root.verify;
                    clicked => { root.verify = true; root.activity(); }
                }
            }
        }
        // Glass of the pill over the black bars (top, left), like the window buttons
        Rectangle {
            x: 0; y: 0;
            width: root.width;
            height: root.video-top;
            clip: true;
            opacity: mode.opacity;
            GlassRing { x: mode.x; y: mode.y; width: mode.width; height: mode.height; }
        }
        Rectangle {
            x: 0; y: 0;
            width: root.video-left;
            height: root.height;
            clip: true;
            opacity: mode.opacity;
            GlassRing { x: mode.x; y: mode.y; width: mode.width; height: mode.height; }
        }

        // Glass circles of the window buttons over the black bars (top, right) or on the home screen:
        // cut to the bar, they extend the glass of mpv, which stops at the edge of the image.
        Rectangle {
            x: 0; y: 0;  // without a position, Slint would center the bar
            width: root.width;
            height: root.has-video ? root.video-top : root.height;
            clip: true;
            opacity: top.opacity;
            for i in [0, 1, 2] : GlassRing {
                x: top.x + i * root.win-step * 1px; y: top.y;
                width: root.win-d * 1px; height: self.width;
            }
        }
        if root.has-video : Rectangle {
            x: root.width - root.video-right; y: 0;
            width: root.video-right;
            height: root.height;
            clip: true;
            opacity: top.opacity;
            for i in [0, 1, 2] : GlassRing {
                x: top.x + i * root.win-step * 1px - parent.x; y: top.y;
                width: root.win-d * 1px; height: self.width;
            }
        }

        // Window buttons: a small glass circle each, hidden with the controls; always visible on the home screen
        top := Rectangle {
            width: top-layout.preferred-width;
            height: root.win-d * 1px;
            x: root.width - self.width - 14px;  // aligned to the window corner, not the image
            y: 14px;
            opacity: root.controls-visible || !root.has-video ? 1 : 0;
            visible: self.opacity > 0;
            animate opacity { duration: 220ms; easing: ease-in-out; }
            TouchArea { changed mouse-x => { root.activity(); } }
            top-layout := HorizontalLayout {
                spacing: (root.win-step - root.win-d) * 1px;
                IconButton {
                    width: root.win-d * 1px; height: self.width; border-radius: self.width / 2;
                    icon-size: 12px; stroke-icon: Icons.minimize;
                    clicked => { root.minimize(); }
                }
                IconButton {
                    width: root.win-d * 1px; height: self.width; border-radius: self.width / 2;
                    icon-size: 12px; stroke-icon: root.is-maximized ? Icons.restore : Icons.maximize;
                    clicked => { root.maximize(); }
                }
                IconButton {
                    width: root.win-d * 1px; height: self.width; border-radius: self.width / 2;
                    icon-size: 12px; stroke-icon: Icons.close; hover-color: #e81123d0;
                    clicked => { root.close-window(); }
                }
            }
        }

        // Resize edges and corners (frameless window)
        if root.resizable : Edge { x: 10px; y: 0; width: parent.width - 20px; height: 5px; hit: 12; mouse-cursor: ns-resize; drag(h) => { root.window-drag(h); } }
        if root.resizable : Edge { x: 10px; y: parent.height - 5px; width: parent.width - 20px; height: 5px; hit: 15; mouse-cursor: ns-resize; drag(h) => { root.window-drag(h); } }
        if root.resizable : Edge { x: 0; y: 10px; width: 5px; height: parent.height - 20px; hit: 10; mouse-cursor: ew-resize; drag(h) => { root.window-drag(h); } }
        if root.resizable : Edge { x: parent.width - 5px; y: 10px; width: 5px; height: parent.height - 20px; hit: 11; mouse-cursor: ew-resize; drag(h) => { root.window-drag(h); } }
        if root.resizable : Edge { x: 0; y: 0; width: 10px; height: 10px; hit: 13; mouse-cursor: nwse-resize; drag(h) => { root.window-drag(h); } }
        if root.resizable : Edge { x: parent.width - 10px; y: 0; width: 10px; height: 10px; hit: 14; mouse-cursor: nesw-resize; drag(h) => { root.window-drag(h); } }
        if root.resizable : Edge { x: 0; y: parent.height - 10px; width: 10px; height: 10px; hit: 16; mouse-cursor: nesw-resize; drag(h) => { root.window-drag(h); } }
        if root.resizable : Edge { x: parent.width - 10px; y: parent.height - 10px; width: 10px; height: 10px; hit: 17; mouse-cursor: nwse-resize; drag(h) => { root.window-drag(h); } }

        // Time under the mouse when hovering the progress bar
        if panel.visible && progress.hovered && root.duration > 0 && !root.verify : Rectangle {
            width: tip-text.preferred-width + 14px;
            height: 22px;
            x: clamp(progress.absolute-position.x + progress.hover-x - self.width / 2, 0, root.width - self.width);
            y: progress.absolute-position.y - 30px;
            border-radius: 6px;
            background: #000000c8;
            tip-text := Text { text: fmt(root.hover-t); color: white; font-size: 12px; }
        }
        // Verify mode: preview of the hovered frame, its number and its timecode
        if panel.visible && progress.hovered && root.duration > 0 && root.verify && !root.precise && root.lift == 0 : Rectangle {
            property <length> img-w: 220px;
            property <length> img-h: root.preview.height > 0 ? self.img-w * root.preview.height / root.preview.width : self.img-w * 9 / 16;
            width: self.img-w + 8px;
            height: self.img-h + 34px;
            x: clamp(progress.absolute-position.x + progress.hover-x - self.width / 2, 4px, root.width - self.width - 4px);
            y: progress.absolute-position.y - self.height - 12px;
            border-radius: 10px;
            background: #000000c8;
            Rectangle {
                x: 4px; y: 4px;
                width: parent.img-w; height: parent.img-h;
                border-radius: 7px;
                clip: true;
                background: #000000;
                Image { width: 100%; height: 100%; source: root.preview; image-fit: contain; }
            }
            Text {
                y: parent.img-h + 8px;
                height: 22px;
                text: @tr("Frame {}", root.hover-frame + 1) + "  ·  " + tc(root.hover-frame);
                color: white; font-size: 12px; font-family: "Consolas";
            }
        }

        // Fine-adjust glass sheet: moving the mouse up, the glass goes from the panel to the full width and the whole
        // bottom of the screen (edges and bottom corners off screen), and rises to make room for the frame row at the top
        sheet := Rectangle {
            property <float> t: root.wide;
            property <length> left: panel.x * (1 - self.t) - 24px * self.t;
            property <length> right: (panel.x + panel.width) * (1 - self.t) + (root.width + 24px) * self.t;
            property <length> top: panel.y - root.lift * root.strip-h;
            property <length> bottom: (panel.y + panel.height) * (1 - self.t) + (root.height + 24px) * self.t;
            x: self.left;
            y: self.top;
            width: self.right - self.left;
            height: self.bottom - self.top;
        }
        if root.lift > 0 && panel.visible : Rectangle {
            x: 0;
            y: sheet.y;
            width: root.width;
            height: max(0px, panel.y - sheet.y);  // between the top of the sheet and the panel: unveils downward
            clip: true;
            opacity: root.lift-soft;
            // ruler: a line across the full width, a tick between each frame, the number of each
            Rectangle { x: 0; y: 30px; width: parent.width; height: 1px; background: #ffffff59; }
            for item in root.strip : Rectangle {
                // continuous position: the row slides under the center of the screen as the mouse advances
                x: (root.width - root.thumb-w) / 2 + (item.frame - root.fine-pos) * root.pitch;
                y: 14px;
                width: root.thumb-w;
                height: parent.height - 14px;
                Text {
                    y: 0; height: 14px;
                    text: item.frame + 1;
                    color: item.current ? white : #ffffffa0;
                    font-size: 11px; font-family: "Consolas";
                    font-weight: item.current ? 600 : 400;
                }
                Rectangle { x: -4px; y: 11px; width: 1px; height: 10px; background: #ffffff80; }
                Rectangle {
                    y: 22px;
                    height: self.width * 9 / 16;
                    border-radius: 6px;
                    clip: true;
                    background: #00000060;
                    border-width: item.current ? 2px : 1px;
                    border-color: item.current ? white : #ffffff40;
                    opacity: item.current ? 1 : 0.8;
                    if item.ready : Image { width: 100%; height: 100%; source: item.image; image-fit: contain; }
                }
                if item.current : Text {
                    y: 22px + parent.width * 9 / 16 + 6px;
                    height: 16px;
                    text: @tr("Frame {}", item.frame + 1) + "  ·  " + tc(item.frame);
                    color: white; font-size: 12px; font-family: "Consolas";
                }
            }
        }

        // Menus: subtitles / audio, or speed (playback settings)
        menu := Rectangle {
            visible: root.menu-open && panel.visible;
            width: root.menu-kind == 2 && root.verify ? 150px : 260px;
            x: min(panel.x + panel.width - self.width, root.width - self.width - 8px);
            y: panel.y - self.height - 10px;
            height: menu-layout.preferred-height;
            TouchArea { }   // absorbs clicks between the entries
            menu-layout := VerticalLayout {
                padding: 10px;
                spacing: 2px;
                if root.menu-kind == 2 : MenuTitle { text: @tr("SPEED"); }
                // Verify mode: Houdini-style draggable value (or type it on click, double-click = 1×)
                if root.menu-kind == 2 && root.verify : HorizontalLayout {
                    padding-left: 4px;
                    padding-top: 2px;
                    padding-bottom: 4px;
                    alignment: start;
                    DragValue {
                        value: root.speed;
                        minimum: 0.01;
                        maximum: 100;
                        editing: root.edit-field == 2;
                        text: root.edit-field == 2 ? root.edit-text + "▏" : round(root.speed * 100) / 100 + "×";
                        edited(v) => { root.set-speed(v); }
                        clicked => { root.begin-edit(2); }
                        reset => { root.begin-edit(0); root.set-speed(1); }
                    }
                }
                if root.menu-kind == 2 && !root.verify : HorizontalLayout {
                    spacing: 4px;
                    for s in [0.5, 0.75, 1, 1.25, 1.5, 2] : Rectangle {
                        height: 26px;
                        border-radius: 6px;
                        background: root.speed == s ? #ffffff40 : s-ta.has-hover ? #ffffff1f : transparent;
                        Text { text: s + "×"; color: white; font-size: 12px; }
                        s-ta := TouchArea { clicked => { root.set-speed(s); } }
                    }
                }
                if root.menu-kind == 1 && root.audio-tracks.length > 0 : MenuTitle { text: @tr("AUDIO"); }
                if root.menu-kind == 1 : VerticalLayout {
                    for t in root.audio-tracks : MenuItem {
                        text: t.label; checked: t.selected;
                        clicked => { root.select-audio(t.id); }
                    }
                }
                if root.menu-kind == 1 : MenuTitle { text: @tr("SUBTITLES"); }
                if root.menu-kind == 1 : VerticalLayout {
                    for t in root.sub-tracks : MenuItem {
                        text: t.label; checked: t.selected;
                        clicked => { root.select-sub(t.id); }
                    }
                }
            }
        }
    }
}
