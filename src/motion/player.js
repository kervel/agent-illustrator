/* agent-illustrator motion player: plays the tracks compiled into an SVG.
 * No dependencies; Web Animations API. The player has no semantics of its
 * own: every value it sets comes from the embedded manifest.
 *
 *   var p = ail.player(svgElement);        // shows frame 0, settled
 *   p.nextStep(); p.prevStep(); p.goToStep(2); p.step; p.steps;
 *   p.next(); p.prev(); p.goTo(3); p.goTo('merge'); p.frame; p.frames;
 *   p.on('frame', function (i, name) { ... }); p.on('step', function (i) { ... });
 *
 * `p.title` / `p.note`: the host metadata in effect (`keyframe "k" [title:
 * "...", note: "..."]`); a title holds until a later frame sets another, a
 * note belongs to its step. Both come with every 'step' event:
 *   p.on('step', function (k, meta) { header.textContent = meta.title; });
 *
 * A step is what one click shows: a frame and the `[auto]` frames that play
 * on after it by themselves. Hosts that advance on clicks use the step API
 * and never need to know which frames are automatic; `p.steps` lists each
 * step's first frame (also `agent-illustrator --list-steps`).
 *
 * Forward plays the segment; backward and jumps are instant and exact.
 */
(function (root) {
  'use strict';
  if (root.ail && root.ail.player) return;

  function camel(p) {
    return p.replace(/-([a-z])/g, function (_, c) { return c.toUpperCase(); });
  }
  function mq(q) {
    return root.matchMedia ? root.matchMedia(q) : { matches: false, addListener: function () {} };
  }

  function Player(svg, opts) {
    opts = opts || {};
    var node = svg.querySelector('script.ail-motion');
    if (!node) throw new Error('ail.player: this SVG has no motion manifest');
    var m = JSON.parse(node.textContent);
    if (m.v !== 1) throw new Error('ail.player: unsupported manifest version ' + m.v);
    var self = this;
    this.svg = svg;
    this.manifest = m;
    this.frames = m.frames.map(function (f) { return f.name; });
    this.steps = [];
    m.frames.forEach(function (f, i) { if (i === 0 || f.auto == null) self.steps.push(i); });
    this.frame = 0;
    this.playing = false;
    this._anims = [];
    this._loops = [];
    this._timers = [];
    this._listeners = {};
    this._nodes = m.channels.map(function (c) {
      return Array.prototype.slice.call(svg.querySelectorAll(c[0]));
    });
    this._reduced = mq('(prefers-reduced-motion: reduce)');
    svg.setAttribute('data-ail-player', '');

    // Print shows the finished story.
    var saved = null;
    function toLast() { saved = self.frame; self.goTo(self.frames.length - 1); }
    function restore() { if (saved !== null) { self.goTo(saved); saved = null; } }
    if (root.addEventListener) {
      root.addEventListener('beforeprint', toLast);
      root.addEventListener('afterprint', restore);
    }

    if (opts.autoplay) {
      this._autoplay(opts.dwell == null ? 1.6 : opts.dwell);
    } else {
      this.goTo(opts.frame == null ? 0 : opts.frame);
    }
  }

  var P = Player.prototype;

  P.on = function (evt, cb) {
    (this._listeners[evt] = this._listeners[evt] || []).push(cb);
    return this;
  };
  P._emit = function (evt) {
    var args = Array.prototype.slice.call(arguments, 1);
    (this._listeners[evt] || []).forEach(function (cb) { cb.apply(null, args); });
  };
  P._index = function (i) {
    if (typeof i === 'string') {
      var k = this.frames.indexOf(i);
      if (k < 0) throw new Error('ail.player: no frame named "' + i + '"');
      return k;
    }
    return Math.max(0, Math.min(this.frames.length - 1, i | 0));
  };
  P._set = function (c, v) {
    var prop = this.manifest.channels[c][1];
    this._nodes[c].forEach(function (n) {
      if (prop === 'text') n.textContent = v;
      else n.style.setProperty(prop, v);
    });
  };
  P._apply = function (row) {
    var vals = this.manifest.settled[row];
    for (var c = 0; c < vals.length; c++) this._set(c, vals[c]);
  };
  P._stop = function () {
    this._anims.concat(this._loops).forEach(function (a) { a.cancel(); });
    this._anims = [];
    this._loops = [];
    this._timers.forEach(clearTimeout);
    this._timers = [];
    this.playing = false;
  };
  P._animate = function (a, loopNow) {
    var self = this;
    var prop = this.manifest.channels[a.c][1];
    if (prop === 'text') {
      a.k.forEach(function (kv) {
        self._timers.push(setTimeout(function () { self._set(a.c, kv[1]); },
          (a.t + kv[0] * a.d) * 1000));
      });
      return;
    }
    var key = camel(prop);
    var frames = a.k.map(function (kv) { var f = { offset: kv[0] }; f[key] = kv[1]; return f; });
    var timing = {
      delay: loopNow ? 0 : a.t * 1000,
      duration: Math.max(a.d * 1000, 1),
      easing: a.e,
      fill: a.loop ? 'none' : 'forwards',
      iterations: a.loop ? Infinity : 1
    };
    this._nodes[a.c].forEach(function (n) {
      if (!n.animate) return;
      var anim = n.animate(frames, timing);
      (a.loop ? self._loops : self._anims).push(anim);
    });
  };
  P._startLoops = function (i) {
    var self = this;
    this.manifest.anims[i].forEach(function (a) { if (a.loop) self._animate(a, true); });
  };

  /** Jump to frame i (index or name): instant, exact. */
  P.goTo = function (i) {
    i = this._index(i);
    this._stop();
    this._apply(i + 1);
    this.frame = i;
    this._startLoops(i);
    this._emit('frame', i, this.frames[i]);
    return this;
  };

  /** Play frame i from the state before it. */
  P.play = function (i) {
    var self = this;
    i = this._index(i);
    this._stop();
    var f = this.manifest.frames[i];
    if (this._reduced.matches || !f.duration) {
      this.goTo(i);
      this._chain(i);
      return this;
    }
    this._apply(i);
    this.frame = i;
    this.playing = true;
    this._emit('framestart', i, this.frames[i]);
    this.manifest.anims[i].forEach(function (a) { self._animate(a, false); });
    this._timers.push(setTimeout(function () {
      self._apply(i + 1);
      self._anims.forEach(function (a) { a.cancel(); });
      self._anims = [];
      self.playing = false;
      self._emit('frame', i, self.frames[i]);
      self._chain(i);
    }, f.duration * 1000 + 20));
    return this;
  };

  P._chain = function (i) {
    var self = this;
    var nf = this.manifest.frames[i + 1];
    if (nf && nf.auto != null) {
      this._timers.push(setTimeout(function () { self.play(i + 1); }, nf.auto * 1000));
    } else if (this._auto) {
      this._auto(i);
    }
  };

  /** The step frame i belongs to. */
  P._stepOf = function (i) {
    var k = 0;
    for (var s = 0; s < this.steps.length; s++) if (this.steps[s] <= i) k = s;
    return k;
  };
  /** Last frame of step k: its own frame and the automatic ones after it. */
  P._stepEnd = function (k) {
    return k + 1 < this.steps.length ? this.steps[k + 1] - 1 : this.frames.length - 1;
  };
  Object.defineProperty(P, 'step', { get: function () { return this._stepOf(this.frame); } });
  /** Host metadata at step k: the latest title set up to its end, its notes. */
  P.meta = function (k) {
    if (k == null) k = this.step;
    var fr = this.manifest.frames, title = null, notes = [];
    for (var i = 0; i <= this._stepEnd(k); i++) if (fr[i].title != null) title = fr[i].title;
    for (var j = this.steps[k]; j <= this._stepEnd(k); j++) if (fr[j].note != null) notes.push(fr[j].note);
    return { title: title, note: notes.length ? notes.join('\n') : null };
  };
  Object.defineProperty(P, 'title', { get: function () { return this.meta().title; } });
  Object.defineProperty(P, 'note', { get: function () { return this.meta().note; } });

  /** Jump to the end of step k (its automatic frames included): instant. */
  P.goToStep = function (k) {
    k = Math.max(0, Math.min(this.steps.length - 1, k | 0));
    this.goTo(this._stepEnd(k));
    this._emit('step', k, this.meta(k));
    return this;
  };

  /** One click: finish the current step, then play the next one through. */
  P.nextStep = function () {
    var k = this.step;
    if (this.playing || this.frame < this._stepEnd(k)) this.goToStep(k);
    if (k + 1 < this.steps.length) {
      this.play(this.steps[k + 1]);
      this._emit('step', k + 1, this.meta(k + 1));
      return true;
    }
    return false;
  };

  /** One click back: the end of the previous step, instantly. */
  P.prevStep = function () {
    var k = this.step;
    if (k > 0) { this.goToStep(k - 1); return true; }
    this.goToStep(0);
    return false;
  };

  /** Advance: play the next frame (finishing the current one first). */
  P.next = function () {
    if (this.playing) this.goTo(this.frame);
    if (this.frame < this.frames.length - 1) { this.play(this.frame + 1); return true; }
    return false;
  };

  /** Step back: instant. */
  P.prev = function () {
    if (this.frame > 0) { this.goTo(this.frame - 1); return true; }
    this.goTo(0);
    return false;
  };

  /** Freeze frame i at t seconds into it (for tests and previews). */
  P.at = function (i, t) {
    var self = this;
    i = this._index(i);
    this._stop();
    this._apply(i);
    this.frame = i;
    this.manifest.anims[i].forEach(function (a) {
      var prop = self.manifest.channels[a.c][1];
      if (prop === 'text') {
        var cur = null;
        a.k.forEach(function (kv) { if (a.t + kv[0] * a.d <= t + 1e-9) cur = kv[1]; });
        if (cur !== null) self._set(a.c, cur);
        return;
      }
      self._animate(a, false);
    });
    this._anims.concat(this._loops).forEach(function (an) { an.pause(); an.currentTime = t * 1000; });
    return this;
  };

  P._autoplay = function (dwell) {
    var self = this;
    this._auto = function (i) {
      self._timers.push(setTimeout(function () {
        if (i < self.frames.length - 1) self.play(i + 1);
        else { self._apply(0); self.play(0); }
      }, dwell * 1000));
    };
    function manual() { self._auto = null; }
    this.svg.addEventListener('click', function () { manual(); if (!self.nextStep()) self.goToStep(0); });
    if (root.document) {
      root.document.addEventListener('keydown', function (e) {
        if (e.key === 'ArrowRight' || e.key === ' ' || e.key === 'PageDown') { manual(); self.nextStep(); e.preventDefault(); }
        else if (e.key === 'ArrowLeft' || e.key === 'PageUp') { manual(); self.prevStep(); e.preventDefault(); }
      });
    }
    this._apply(0);
    this.play(0);
  };

  root.ail = root.ail || {};
  root.ail.player = function (svg, opts) {
    if (!svg.__ailPlayer) svg.__ailPlayer = new Player(svg, opts);
    return svg.__ailPlayer;
  };
})(typeof window !== 'undefined' ? window : this);
