// The two views, and where parker's face sits in each.
//
// This file is the *placement* half of the pose/placement split, which is the
// one rule that keeps a face app from becoming a second face implementation:
//
//   pose      — which dots are lit, in what ink, with what motion.
//               parker has one at a time and every surface reflects it.
//               parkerd owns it. visage never invents one.
//   placement — where that grid lands on *this* screen and how big.
//               The Kraken LCD has no such concept. visage owns it alone.
//
// The test: anything expressible as (dot position, time) -> screen position
// belongs here. Anything that changes what the face *is* belongs in parkerd.
// The river travel (lens.js) is the proof by construction — the face pours
// itself across the screen dot by dot, and parkerd never hears about it.
//
// Two postures, and the difference between them is how much of the screen
// parker is holding:
//
//   default   parker over the transcript's tail, the input low. Where you
//             land, and where you type — the old `home` and `prompt` merged.
//   session   parker down the left rail, the conversation across the rest.
//
// There was a third, `deep`, and it is gone (Aug 2026, DECISIONS.md): it
// moved parker without giving the transcript the space that move was for, so
// the far end of the axis bought a smaller face and nothing else. Two views
// are not an axis, they are a toggle — hence Ctrl+Tab and not Ctrl+±.
//
// parker is smaller in `session` than in `default`, and both views are
// *fixed*: the old `prompt` lean-back is gone, so layout moves because you
// asked and never because the content grew.

export const VIEWS = ['default', 'session'];

/// The other one. A toggle has no direction and no ends, so there is nothing
/// here to walk off: whatever you are in, this is the one you are not.
export function other(view) {
  return view === 'session' ? 'default' : 'session';
}

// The grid is 96x40 and the renderer pads it to 100x44 before fitting, but
// the poses only ever light the middle: an iris ring reaches ~19 dots from
// centre and the speaking waveform is 46 wide, so what you actually *see* is
// about 46x40 dots. Sizing boxes by the padded grid is why every view but
// the old `home` came out too small — a 320px rail drew a 115px parker and
// read as a coin. These constants are the difference between asking for a
// box and asking for a face of a given size.
const SEEN_W = 46;
const SEEN_H = 40;
const GRID_W = 100;
const GRID_H = 44;

/// The box that renders parker at roughly `w` x `h` pixels of *visible* face.
/// Fits the way the renderer does — whichever axis binds first — so the
/// result never exceeds either bound.
///
/// The box comes back much larger than the face and is allowed to overhang
/// the screen: only lit dots show, `main` clips, and the canvas takes no
/// pointer events. On 16:9 `default` binds vertically and lands a face about
/// 950px wide — the width fraction is a box, not a face width; treating it
/// as a face width is the mistake that clips the top of the iris.
function faceBox(w, h) {
  const pitch = Math.min(w / SEEN_W, h / SEEN_H);
  return { w: GRID_W * pitch, h: GRID_H * pitch };
}

/// Where the face box sits, in viewport pixels, centre-anchored.
/// Numbers are the redesign's (docs/visage/redesign/README.md, "Placement").
export function placement(view, vw, vh) {
  switch (view) {
    // A rail to parker, the conversation across the rest. He goes left
    // because the tower is left of the monitor — near enough to touch, so
    // that is the side he arrives from.
    case 'session':
      return { cx: vw * 0.165, cy: vh * 0.5, ...faceBox(vw * 0.28, vh * 0.52) };

    // parker over the top third, the transcript's tail beneath him, the
    // input low. Fixed: he does not lean back when a conversation starts.
    case 'default':
    default:
      return { cx: vw * 0.5, cy: vh * 0.3, ...faceBox(vw * 0.62, vh * 0.58) };
  }
}
