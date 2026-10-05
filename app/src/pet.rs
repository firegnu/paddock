//! A decorative pet patrolling the spare tab-strip space, drawn from a pixel image pack. No agent
//! state or input. From Saddle `src/mascot.rs` at commit `df1c727`: the pack format and the patrol
//! (walk, act, turn at the ends, flip or `-left` clips when heading left, 12 fps, no catch-up),
//! without the block-glyph packs and their drawing, the Kitty image protocol and the display
//! choice: paddock always draws the pictures. The lane is counted in steps rather than terminal
//! cells; a step is a sixteenth of the pet's drawn width, as a cell was in Saddle.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    sync::{Arc, LazyLock},
};

const FPS: f64 = 12.0;
/// Steps across the pet's own width, as Saddle's canvas was 16 cells wide.
pub const STEPS_PER_WIDTH: u32 = 16;

pub type Rgb = (u8, u8, u8);

/// The built-in pets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pet {
    #[default]
    Clawd,
    Cat,
    Capybara,
}

impl Pet {
    pub const ALL: [Pet; 3] = [Pet::Clawd, Pet::Cat, Pet::Capybara];

    /// How the pet is written in the config file.
    pub fn name(self) -> &'static str {
        match self {
            Pet::Clawd => "clawd",
            Pet::Cat => "cat",
            Pet::Capybara => "capybara",
        }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        Self::ALL
            .into_iter()
            .find(|p| p.name() == value)
            .ok_or_else(|| format!("unknown mascot {value:?}: expected clawd, cat or capybara"))
    }

    fn pack(self) -> Arc<Pack> {
        // Built-in packs are checked in and covered by tests; nothing is read from disk.
        static PACKS: LazyLock<[Arc<Pack>; 3]> = LazyLock::new(|| {
            [
                include_str!("../assets/pets/clawd-image.toml"),
                include_str!("../assets/pets/cat-image.toml"),
                include_str!("../assets/pets/capybara-image.toml"),
            ]
            .map(|text| Arc::new(Pack::parse(text).expect("built-in pet pack")))
        });
        PACKS[self as usize].clone()
    }
}

impl<'de> Deserialize<'de> for Pet {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Pet::parse(&value).map_err(serde::de::Error::custom)
    }
}

struct Clip {
    name: String,
    /// The pose shown at each 12 fps tick.
    ticks: Vec<usize>,
    /// The clip drawn for heading left, when the pack has `<name>-left`.
    left: Option<usize>,
}

/// A pack's pictures and clips.
pub struct Pack {
    /// Index 0 is unused: it stands for transparent.
    palette: Vec<Rgb>,
    pub width: usize,
    pub height: usize,
    /// One palette index per pixel, row by row.
    poses: Vec<Vec<u8>>,
    clips: Vec<Clip>,
    walking: usize,
    turning: usize,
    /// Clips played between walks.
    actions: Vec<usize>,
    /// Poses without a `-left` clip are flipped when heading left.
    mirror: bool,
    /// Ticks per step of travel; the walking clip changes pose on the same ticks.
    step_ticks: usize,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    #[serde(default = "default_step")]
    step_ticks: usize,
    #[serde(default)]
    mirror: bool,
    /// `[width, height]` of the pixel images.
    size: Option<(usize, usize)>,
    palette: BTreeMap<char, String>,
    poses: BTreeMap<String, PoseSource>,
    clip: Vec<ClipSource>,
}
fn default_step() -> usize {
    4
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PoseSource {
    /// `size` rows of palette letters, one per pixel.
    pixels: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClipSource {
    name: String,
    /// `[pose, ticks]` in order.
    frames: Vec<(String, usize)>,
}

impl Pack {
    pub fn parse(text: &str) -> Result<Self> {
        let source: Source = toml::from_str(text)?;
        ensure!(source.step_ticks > 0, "step_ticks must be positive");
        let (width, height) = source
            .size
            .context("a pet pack needs its size: paddock draws pets as pixel images")?;
        ensure!(width > 0 && height > 0, "size must be positive");
        let mut palette = vec![(0, 0, 0)];
        let mut letters = BTreeMap::from([('.', 0u8)]);
        for (&letter, hex) in &source.palette {
            let rgb = u32::from_str_radix(hex.trim_start_matches('#'), 16)
                .ok()
                .filter(|_| hex.len() == 7)
                .with_context(|| format!("palette {letter}: expected #rrggbb, got {hex:?}"))?;
            letters.insert(letter, u8::try_from(palette.len())?);
            palette.push(((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8));
        }
        let mut names = BTreeMap::new();
        let mut poses = Vec::new();
        for (name, pose) in &source.poses {
            names.insert(name.as_str(), names.len());
            let pixels = pose
                .pixels
                .as_deref()
                .with_context(|| format!("pose {name}: missing pixels"))?;
            let rows: Vec<_> = pixels.lines().filter(|row| !row.is_empty()).collect();
            ensure!(
                rows.len() == height && rows.iter().all(|row| row.chars().count() == width),
                "pose {name}: pixels must be {height} rows of {width} letters"
            );
            let image = rows.iter().flat_map(|row| row.chars()).map(|c| {
                letters
                    .get(&c)
                    .copied()
                    .with_context(|| format!("pose {name}: unknown color {c:?}"))
            });
            poses.push(image.collect::<Result<_>>()?);
        }
        let mut clips = Vec::new();
        for clip in &source.clip {
            let mut ticks = Vec::new();
            for (pose, count) in &clip.frames {
                let index = *names
                    .get(pose.as_str())
                    .with_context(|| format!("clip {}: unknown pose {pose:?}", clip.name))?;
                ticks.extend(std::iter::repeat_n(index, *count));
            }
            ensure!(!ticks.is_empty(), "clip {}: no frames", clip.name);
            clips.push(Clip {
                name: clip.name.clone(),
                ticks,
                left: None,
            });
        }
        let find = |name: &str| clips.iter().position(|c| c.name == name);
        let lefts: Vec<_> = clips
            .iter()
            .map(|c| find(&format!("{}-left", c.name)))
            .collect();
        for (clip, &left) in clips.iter().zip(&lefts) {
            ensure!(
                left.is_none_or(|l| clips[l].ticks.len() == clip.ticks.len()),
                "clip {}-left must last as long as {}",
                clip.name,
                clip.name
            );
        }
        let walking = find("walking").context("missing the walking clip")?;
        let turning = find("turning").context("missing the turning clip")?;
        ensure!(
            clips[walking].ticks.len() % source.step_ticks == 0,
            "the walking clip must be a whole number of steps"
        );
        let actions: Vec<_> = (0..clips.len())
            .filter(|&i| i != walking && i != turning && !clips[i].name.ends_with("-left"))
            .collect();
        ensure!(!actions.is_empty(), "no action clips");
        for (clip, left) in clips.iter_mut().zip(lefts) {
            clip.left = left;
        }
        Ok(Self {
            palette,
            width,
            height,
            poses,
            clips,
            walking,
            turning,
            actions,
            mirror: source.mirror,
            step_ticks: source.step_ticks,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Motion {
    Walking,
    Acting(usize),
    Turning,
}

/// The pet and where it is in its patrol.
pub struct Mascot {
    pack: Arc<Pack>,
    last: Option<f64>,
    /// Seconds into the current motion.
    phase: f64,
    /// Steps from the left end of the lane.
    x: u32,
    right: bool,
    motion: Motion,
    /// Steps already taken, and ticks to walk, in the current walk.
    stepped: usize,
    walk_ticks: usize,
    random: u64,
    recent: Vec<usize>,
}

/// One horizontal run of same-coloured pixels in the picture as drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub x: usize,
    pub y: usize,
    pub len: usize,
    pub color: Rgb,
}

impl Mascot {
    pub fn new(pet: Pet) -> Self {
        Self::with(pet.pack())
    }

    /// A pet from pack text, as documented in `assets/pets/README.md`.
    pub fn from_pack(text: &str) -> Result<Self> {
        Ok(Self::with(Arc::new(Pack::parse(text)?)))
    }

    fn with(pack: Arc<Pack>) -> Self {
        let mut mascot = Self {
            pack,
            last: None,
            phase: 0.0,
            x: 0,
            right: true,
            motion: Motion::Walking,
            stepped: 0,
            walk_ticks: 0,
            random: 0,
            recent: Vec::new(),
        };
        mascot.walk(3.0);
        mascot
    }

    pub fn pack(&self) -> &Pack {
        &self.pack
    }

    /// Steps from the left end of the lane.
    pub fn x(&self) -> u32 {
        self.x
    }

    /// Not shown: the next frame starts afresh instead of catching up.
    pub fn hide(&mut self) {
        self.last = None;
    }

    fn random(&mut self) -> f64 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        (self.random >> 11) as f64 / ((1u64 << 53) as f64)
    }

    /// Walks for about `seconds`, ending on a whole walk cycle so the feet finish together.
    fn walk(&mut self, seconds: f64) {
        let cycle = self.pack.clips[self.pack.walking].ticks.len();
        self.motion = Motion::Walking;
        self.phase = 0.0;
        self.stepped = 0;
        self.walk_ticks = ((seconds * FPS / cycle as f64).ceil() as usize).max(1) * cycle;
    }

    fn act(&mut self) {
        let choices: Vec<_> = self
            .pack
            .actions
            .iter()
            .copied()
            .filter(|i| !self.recent.contains(i))
            .collect();
        // A small pack can have fewer actions than the recent list holds.
        let choices = if choices.is_empty() {
            self.recent.clear();
            self.pack.actions.clone()
        } else {
            choices
        };
        let index = choices[(self.random() * choices.len() as f64) as usize];
        self.recent.push(index);
        if self.recent.len() > 5 {
            self.recent.remove(0);
        }
        self.phase = 0.0;
        self.motion = Motion::Acting(index);
    }

    /// Whole ticks into the current motion. Frame times are sums of fractions, so a tick
    /// boundary reached "exactly" must not read as the tick before it.
    fn tick(&self) -> usize {
        (self.phase * FPS + 1e-6) as usize
    }

    /// Moves the patrol on to `now` (seconds) in a lane `limit` steps long.
    pub fn advance(&mut self, now: f64, limit: u32) {
        if self.random == 0 {
            self.random = now.to_bits() | 1;
        }
        // No catch-up after a hidden view, suspend, clock jump, or slow frame.
        let dt = self.last.map_or(0.0, |last| (now - last).clamp(0.0, 0.25));
        self.last = Some(now);
        self.phase += dt;
        self.x = self.x.min(limit);
        match self.motion {
            Motion::Walking => {
                // The body moves a step on the same ticks the walking clip changes pose.
                while self.stepped < self.tick() / self.pack.step_ticks {
                    self.stepped += 1;
                    if self.right && self.x < limit {
                        self.x += 1;
                    } else if !self.right && self.x > 0 {
                        self.x -= 1;
                    }
                    if limit > 0 && self.x == if self.right { limit } else { 0 } {
                        self.motion = Motion::Turning;
                        self.phase = 0.0;
                        return;
                    }
                }
                if self.tick() >= self.walk_ticks {
                    self.act();
                }
            }
            Motion::Acting(index) => {
                if self.tick() >= self.pack.clips[index].ticks.len() {
                    let seconds = 2.2 + self.random() * 2.4;
                    self.walk(seconds);
                }
            }
            Motion::Turning => {
                let length = self.pack.clips[self.pack.turning].ticks.len();
                if self.tick() >= length {
                    self.right = self.heading();
                    let seconds = 2.2 + self.random() * 2.4;
                    self.walk(seconds);
                }
            }
        }
    }

    /// Whether the pet faces right now: a turn changes its facing halfway through.
    fn heading(&self) -> bool {
        match self.motion {
            Motion::Turning
                if self.tick() >= self.pack.clips[self.pack.turning].ticks.len().div_ceil(2) =>
            {
                !self.right
            }
            _ => self.right,
        }
    }

    /// The pose to draw and whether to flip it.
    pub fn pose(&self) -> (usize, bool) {
        let pack = &self.pack;
        let tick = self.tick();
        let (index, tick) = match self.motion {
            Motion::Walking => (pack.walking, tick % pack.clips[pack.walking].ticks.len()),
            Motion::Turning => (pack.turning, tick),
            Motion::Acting(index) => (index, tick),
        };
        let (index, flip) = match (self.heading(), pack.clips[index].left) {
            (true, _) => (index, false),
            (false, Some(left)) => (left, false),
            (false, None) => (index, pack.mirror),
        };
        let ticks = &pack.clips[index].ticks;
        (ticks[tick.min(ticks.len() - 1)], flip)
    }

    /// The current picture as runs of same-coloured pixels, flipped when it should be.
    pub fn runs(&self) -> Vec<Run> {
        let (pose, flip) = self.pose();
        let pack = &self.pack;
        let image = &pack.poses[pose];
        let mut runs = Vec::new();
        for y in 0..pack.height {
            let row = &image[y * pack.width..(y + 1) * pack.width];
            let at = |x: usize| row[if flip { pack.width - 1 - x } else { x }];
            let mut x = 0;
            while x < pack.width {
                let color = at(x);
                let start = x;
                while x < pack.width && at(x) == color {
                    x += 1;
                }
                if color != 0 {
                    runs.push(Run {
                        x: start,
                        y,
                        len: x - start,
                        color: pack.palette[usize::from(color)],
                    });
                }
            }
        }
        runs
    }
}

/// The pet in the tab strip's spare room: it learns the room's width when painted, walks it, and
/// asks to be drawn again only when its pose or place changes.
pub struct PetView {
    mascot: Mascot,
    /// The lane in steps, and whether the pet fits at all, as of the last paint.
    lane: Option<u32>,
    /// What was last drawn: place and pose.
    drawn: Option<(u32, (usize, bool))>,
}

impl PetView {
    pub fn new(pet: Pet, cx: &mut gpui::Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs_f64(1.0 / FPS))
                    .await;
                if this.update(cx, |view, cx| view.tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            mascot: Mascot::new(pet),
            lane: None,
            drawn: None,
        }
    }

    fn tick(&mut self, cx: &mut gpui::Context<Self>) {
        let Some(lane) = self.lane else {
            self.mascot.hide();
            return;
        };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0.0, |d| d.as_secs_f64());
        self.mascot.advance(now, lane);
        let state = (self.mascot.x(), self.mascot.pose());
        if self.drawn != Some(state) {
            self.drawn = Some(state);
            cx.notify();
        }
    }

    /// Whole screen points per picture pixel, so the pixel art stays crisp, as large as fits.
    fn scale(&self, height: f32) -> f32 {
        (height / self.mascot.pack().height as f32).floor().max(1.0)
    }
}

impl gpui::Render for PetView {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        use gpui::{Styled as _, point, px, size};
        let entity = cx.entity();
        let runs = self.mascot.runs();
        let x = self.mascot.x();
        let (width, height) = (self.mascot.pack().width, self.mascot.pack().height);
        gpui::canvas(
            move |bounds, _, cx| {
                entity.update(cx, |view, _| {
                    let scale = view.scale(f32::from(bounds.size.height));
                    let drawn = width as f32 * scale;
                    let step = drawn / STEPS_PER_WIDTH as f32;
                    let room = f32::from(bounds.size.width) - drawn;
                    view.lane = (room >= 0.0).then(|| (room / step) as u32);
                    (scale, step, view.lane.is_some())
                })
            },
            move |bounds, (scale, step, fits), window, _| {
                if !fits {
                    return;
                }
                let left = bounds.origin.x + px(x as f32 * step);
                let top = bounds.origin.y + bounds.size.height - px(height as f32 * scale);
                for run in &runs {
                    let (r, g, b) = run.color;
                    let color = gpui::Rgba {
                        r: f32::from(r) / 255.0,
                        g: f32::from(g) / 255.0,
                        b: f32::from(b) / 255.0,
                        a: 1.0,
                    };
                    window.paint_quad(gpui::fill(
                        gpui::Bounds::new(
                            point(
                                left + px(run.x as f32 * scale),
                                top + px(run.y as f32 * scale),
                            ),
                            size(px(run.len as f32 * scale), px(scale)),
                        ),
                        color,
                    ));
                }
            },
        )
        .size_full()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pack text with one 2x2 pose `p`, the walking, turning and an `act` clip, plus `extra`.
    fn text(size: &str, pose: &str, extra: &str) -> String {
        format!(
            "{size}[palette]\nA = \"#ff0000\"\nB = \"#00ff00\"\n[poses.p]\n{pose}\n\
             [[clip]]\nname = \"walking\"\nframes = [[\"p\", 4]]\n\
             [[clip]]\nname = \"turning\"\nframes = [[\"p\", 1]]\n\
             [[clip]]\nname = \"act\"\nframes = [[\"p\", 1]]\n{extra}"
        )
    }

    const PIXELS: &str = "pixels = '''\nAB\n.A\n'''";

    #[test]
    fn broken_packs_are_refused_with_the_reason() {
        assert!(Pack::parse(&text("size = [2, 2]\n", PIXELS, "")).is_ok());
        let reason = |size: &str, pose: &str| {
            Pack::parse(&text(size, pose, ""))
                .err()
                .unwrap()
                .to_string()
        };
        assert!(reason("size = [3, 2]\n", PIXELS).contains("2 rows of 3 letters"));
        assert!(reason("size = [2, 2]\n", "pixels = '''\nA.\n.Z\n'''").contains("unknown color"));
        assert!(reason("", PIXELS).contains("needs its size"));
        assert!(reason("size = [0, 2]\n", PIXELS).contains("positive"));
        // Saddle's block-glyph poses are not a format paddock reads.
        assert!(!reason("size = [2, 2]\n", "art = 'x'").is_empty());
    }

    #[test]
    fn left_clips_pair_with_their_clip_and_are_never_actions() {
        let left = |ticks| format!("[[clip]]\nname = \"act-left\"\nframes = [[\"p\", {ticks}]]\n");
        let pack = Pack::parse(&text("size = [2, 2]\n", PIXELS, &left(1))).unwrap();
        let names: Vec<_> = pack
            .actions
            .iter()
            .map(|&i| pack.clips[i].name.as_str())
            .collect();
        assert_eq!(names, ["act"]);
        assert_eq!(pack.clips[pack.actions[0]].left, Some(3));
        let long = Pack::parse(&text("size = [2, 2]\n", PIXELS, &left(2)));
        assert!(
            long.err()
                .unwrap()
                .to_string()
                .contains("must last as long")
        );
    }

    /// Runs the patrol tick by tick from `start`, for `ticks` ticks, in a lane of `limit` steps.
    fn play(m: &mut Mascot, start: u32, ticks: u32, limit: u32) -> Vec<(u32, usize, bool)> {
        (start..start + ticks)
            .map(|t| {
                m.advance(100.0 + f64::from(t) / FPS, limit);
                let (pose, flip) = m.pose();
                (m.x(), pose, flip)
            })
            .collect()
    }

    #[test]
    fn walking_moves_one_step_exactly_when_the_feet_change() {
        // Two poses alternating every 2 ticks; a step every 2 ticks.
        let pack = "step_ticks = 2\nsize = [1, 1]\n[palette]\nA = \"#ff0000\"\n\
            [poses.a]\npixels = 'A'\n[poses.b]\npixels = 'A'\n\
            [[clip]]\nname = \"walking\"\nframes = [[\"a\", 2], [\"b\", 2]]\n\
            [[clip]]\nname = \"turning\"\nframes = [[\"a\", 2]]\n\
            [[clip]]\nname = \"act\"\nframes = [[\"a\", 1]]\n";
        let mut m = Mascot::from_pack(pack).unwrap();
        let frames = play(&mut m, 0, 9, 100);
        let xs: Vec<_> = frames.iter().map(|f| f.0).collect();
        assert_eq!(xs, [0, 0, 1, 1, 2, 2, 3, 3, 4]);
        // The pose changes on the same ticks as the position.
        let poses: Vec<_> = frames.iter().map(|f| f.1).collect();
        assert_eq!(poses, [0, 0, 1, 1, 0, 0, 1, 1, 0]);
    }

    #[test]
    fn the_lane_end_turns_the_pet_halfway_through_the_turn() {
        let pack = "step_ticks = 1\nmirror = true\nsize = [1, 1]\n[palette]\nA = \"#ff0000\"\n\
            [poses.a]\npixels = 'A'\n\
            [[clip]]\nname = \"walking\"\nframes = [[\"a\", 1]]\n\
            [[clip]]\nname = \"turning\"\nframes = [[\"a\", 4]]\n\
            [[clip]]\nname = \"act\"\nframes = [[\"a\", 1]]\n";
        let mut m = Mascot::from_pack(pack).unwrap();
        let frames = play(&mut m, 0, 8, 2);
        // Walks right to the end of a two-step lane, then turns: flipped from halfway on.
        assert_eq!(frames[2].0, 2);
        let flips: Vec<_> = frames[2..7].iter().map(|f| f.2).collect();
        assert_eq!(flips, [false, false, true, true, true]);
        // And then walks back left.
        let later = play(&mut m, 8, 3, 2);
        assert!(later.last().unwrap().0 < 2, "{later:?}");
    }

    #[test]
    fn a_left_clip_replaces_mirroring() {
        let pack = "step_ticks = 1\nmirror = true\nsize = [2, 1]\n[palette]\nA = \"#ff0000\"\n\
            [poses.a]\npixels = 'A.'\n[poses.l]\npixels = '.A'\n\
            [[clip]]\nname = \"walking\"\nframes = [[\"a\", 1]]\n\
            [[clip]]\nname = \"walking-left\"\nframes = [[\"l\", 1]]\n\
            [[clip]]\nname = \"turning\"\nframes = [[\"a\", 2]]\n\
            [[clip]]\nname = \"act\"\nframes = [[\"a\", 1]]\n";
        let mut m = Mascot::from_pack(pack).unwrap();
        // One step to the end of the lane, a two-tick turn, then the first tick heading left.
        play(&mut m, 0, 4, 1);
        // Heading left now: the `-left` pose, not a flipped one.
        assert_eq!(m.pose(), (1, false));
    }

    #[test]
    fn runs_merge_colours_and_follow_the_flip() {
        let mut m = Mascot::from_pack(&text("mirror = true\nsize = [2, 2]\n", PIXELS, "")).unwrap();
        let red = (0xff, 0, 0);
        let green = (0, 0xff, 0);
        assert_eq!(
            m.runs(),
            [
                Run {
                    x: 0,
                    y: 0,
                    len: 1,
                    color: red
                },
                Run {
                    x: 1,
                    y: 0,
                    len: 1,
                    color: green
                },
                Run {
                    x: 1,
                    y: 1,
                    len: 1,
                    color: red
                },
            ]
        );
        m.right = false;
        assert_eq!(
            m.runs()[0],
            Run {
                x: 0,
                y: 0,
                len: 1,
                color: green
            }
        );
    }

    #[test]
    fn every_built_in_pet_parses_and_patrols() {
        for pet in Pet::ALL {
            let mut m = Mascot::new(pet);
            assert!(m.pack().width > 0 && m.pack().height > 0, "{pet:?}");
            // A minute of patrol in a 40-step lane: it acts, turns, and stays in the lane.
            let mut acted = false;
            for t in 0..(60.0 * FPS) as u32 {
                m.advance(f64::from(t) / FPS, 40);
                assert!(m.x() <= 40);
                acted |= matches!(m.motion, Motion::Acting(_));
                assert!(!m.runs().is_empty(), "{pet:?} drew nothing");
            }
            assert!(acted, "{pet:?} never acted");
        }
        assert_eq!(Pet::parse("cat"), Ok(Pet::Cat));
        assert!(
            Pet::parse("dog")
                .unwrap_err()
                .contains("clawd, cat or capybara")
        );
    }

    #[test]
    fn recent_actions_are_not_repeated() {
        let mut m = Mascot::new(Pet::Clawd);
        let mut seen = Vec::new();
        for _ in 0..6 {
            m.act();
            let Motion::Acting(index) = m.motion else {
                unreachable!()
            };
            seen.push(index);
        }
        // Clawd has more than five actions, so six picks in a row are all different.
        let mut unique = seen.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), 6, "{seen:?}");
    }
}
