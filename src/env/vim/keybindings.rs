//! # Vim Keybindings
//!
//! ## Overview
//!
//! This module handles mapping the keybindings used in Vim onto the
//! [Action](crate::editing::base::Action) type.
//!
//! ## Example
//!
//! ```
//! use modalkit::env::vim::VimMode;
//! use modalkit::env::vim::keybindings::VimMachine;
//!
//! use modalkit::editing::base::{Count, Resolve};
//! use modalkit::editing::base::{Action, EditAction, EditTarget, HistoryAction, RangeType};
//!
//! use modalkit::input::{bindings::BindingMachine, key::TerminalKey};
//!
//! use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
//!
//! const fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
//!     KeyEvent::new(code, modifiers)
//! }
//!
//! fn main() {
//!     let mut keybindings: VimMachine<TerminalKey> = Default::default();
//!
//!     // Begins in Normal mode.
//!     assert_eq!(keybindings.mode(), VimMode::Normal);
//!
//!     // Typing "5dd" deletes a line.
//!     keybindings.input_key(key(KeyCode::Char('5'), KeyModifiers::NONE).into());
//!     keybindings.input_key(key(KeyCode::Char('d'), KeyModifiers::NONE).into());
//!     keybindings.input_key(key(KeyCode::Char('d'), KeyModifiers::NONE).into());
//!
//!     let (act, ctx) = keybindings.pop().unwrap();
//!     assert_eq!(act, Action::Edit(EditAction::Delete.into(), RangeType::Line.into()));
//!     assert_eq!(ctx.resolve(&Count::Contextual), 5);
//!
//!     // Returning to Normal mode causes history checkpoint.
//!     let (act, _) = keybindings.pop().unwrap();
//!     assert_eq!(act, HistoryAction::Checkpoint.into());
//!
//!     // End of available actions.
//!     assert_eq!(keybindings.pop(), None);
//! }
//! ```
use bitflags::bitflags;

use crate::editing::base::{
    Action,
    Application,
    Axis,
    Case,
    Char,
    CloseFlags,
    CloseTarget,
    CommandAction,
    CommandBarAction,
    CommandType,
    Count,
    CursorAction,
    CursorEnd,
    EditAction,
    EditTarget,
    FocusChange,
    HistoryAction,
    IndentChange,
    InsertStyle,
    InsertTextAction,
    JoinStyle,
    MacroAction,
    MoveDir1D,
    MoveDir2D,
    MoveDirMod,
    MovePosition,
    MoveTerminus,
    MoveType,
    NumberChange,
    PositionList,
    RangeType,
    Register,
    RepeatType,
    ScrollSize,
    ScrollStyle,
    SearchType,
    SelectionAction,
    SelectionCursorChange,
    SelectionResizeStyle,
    SelectionSplitStyle,
    SizeChange,
    Specifier,
    TabAction,
    TargetShape,
    TargetShapeFilter,
    WindowAction,
    WordStyle,
};

use super::{
    super::{keyparse::parse, CommonKeyClass},
    VimContext,
    VimMode,
};

use crate::input::{
    bindings::{InputBindings, ModalMachine, Step},
    key::TerminalKey,
};

bitflags! {
    struct MappedModes: u32 {
        const N = 0b0000000000000001;
        const X = 0b0000000000000010;
        const S = 0b0000000000000100;
        const O = 0b0000000000001000;
        const I = 0b0000000000010000;
        const C = 0b0000000000100000;
        const L = 0b0000000001000000;

        const SUFFIX_CHARSRCH = 0b1000000000000000;
        const SUFFIX_CHARREPL = 0b0100000000000000;

        const V = MappedModes::X.bits | MappedModes::S.bits;

        const NVI = MappedModes::N.bits | MappedModes::V.bits | MappedModes::I.bits;
        const NVO = MappedModes::N.bits | MappedModes::V.bits | MappedModes::O.bits;
        const NXO = MappedModes::N.bits | MappedModes::X.bits | MappedModes::O.bits;
        const NV = MappedModes::N.bits | MappedModes::V.bits;
        const NX = MappedModes::N.bits | MappedModes::X.bits;
        const VO = MappedModes::V.bits | MappedModes::O.bits;
        const IC = MappedModes::I.bits | MappedModes::C.bits;
    }
}

const MAP: MappedModes = MappedModes::NVO;
const NVMAP: MappedModes = MappedModes::NV;
const NXMAP: MappedModes = MappedModes::NX;
const NXOMAP: MappedModes = MappedModes::NXO;
const NVIMAP: MappedModes = MappedModes::NVI;
const VOMAP: MappedModes = MappedModes::VO;
const ICMAP: MappedModes = MappedModes::IC;

const NMAP: MappedModes = MappedModes::N;
const OMAP: MappedModes = MappedModes::O;
const IMAP: MappedModes = MappedModes::I;
const CMAP: MappedModes = MappedModes::C;
const VMAP: MappedModes = MappedModes::V;
const XMAP: MappedModes = MappedModes::X;
const SMAP: MappedModes = MappedModes::S;

const SUFFIX_CHARREPL: MappedModes = MappedModes::SUFFIX_CHARREPL;
const SUFFIX_CHARSRCH: MappedModes = MappedModes::SUFFIX_CHARSRCH;

impl MappedModes {
    pub fn split(&self) -> Vec<VimMode> {
        let mut modes = Vec::new();

        if self.contains(MappedModes::N) {
            modes.push(VimMode::Normal);
        }

        if self.contains(MappedModes::X) {
            modes.push(VimMode::Visual);
        }

        if self.contains(MappedModes::S) {
            modes.push(VimMode::Select);
        }

        if self.contains(MappedModes::O) {
            modes.push(VimMode::OperationPending);
        }

        if self.contains(MappedModes::I) {
            modes.push(VimMode::Insert);
        }

        if self.contains(MappedModes::L) {
            modes.push(VimMode::LangArg);
        }

        if self.contains(MappedModes::C) {
            modes.push(VimMode::Command);
        }

        if self.contains(MappedModes::SUFFIX_CHARREPL) {
            modes.push(VimMode::CharReplaceSuffix);
        }

        if self.contains(MappedModes::SUFFIX_CHARSRCH) {
            modes.push(VimMode::CharSearchSuffix);
        }

        return modes;
    }
}

#[derive(Clone, Debug)]
enum InternalAction {
    SetCursorEnd(CursorEnd),
    SetTarget(EditTarget),
    SetSearchCharParams(MoveDir1D, bool),
    SetSearchChar,
    SetSearchRegexParams(MoveDir1D),
    SetRegister(Register),
    SetReplaceChar(Option<Char>),
    SaveCounting,
    SetCursorChar(char),
    SetCursorDigraph,
    SetInsertStyle(InsertStyle),
    SetTargetShape(TargetShapeFilter, TargetShape),
    SetOperation(EditAction),
    SetPostMode(VimMode),
}

impl InternalAction {
    pub fn run<P: Application>(&self, ctx: &mut VimContext<P>) {
        match self {
            InternalAction::SetCursorEnd(end) => {
                ctx.action.cursor_end = Some(*end);
            },
            InternalAction::SetSearchCharParams(dir, inclusive) => {
                ctx.action.charsearch_params = Some((*dir, *inclusive));
            },
            InternalAction::SetSearchChar => {
                if let Some((d, i)) = ctx.action.charsearch_params.take() {
                    ctx.persist.charsearch_params = (d, i);
                }

                ctx.persist.charsearch = ctx.ch.get_typed();
            },
            InternalAction::SetSearchRegexParams(dir) => {
                ctx.persist.regexsearch_dir = *dir;
            },
            InternalAction::SetRegister(reg) => {
                ctx.action.register = Some(*reg);
            },
            InternalAction::SetReplaceChar(c) => {
                if c.is_some() {
                    ctx.action.replace = c.clone();
                } else {
                    ctx.action.replace = ctx.ch.get_typed();
                }
            },
            InternalAction::SaveCounting => {
                match ctx.action.count {
                    None => {
                        ctx.action.count = ctx.action.counting;
                        ctx.action.counting = None;
                    },
                    Some(prev) => {
                        ctx.action.count =
                            Some(prev.saturating_mul(ctx.action.counting.unwrap_or(1)));
                        ctx.action.counting = None;
                    },
                }
            },
            InternalAction::SetCursorChar(c) => {
                ctx.action.cursor = Some(*c);
            },
            InternalAction::SetCursorDigraph => {
                if ctx.ch.digraph1.is_some() {
                    ctx.action.cursor = ctx.ch.digraph1;
                }
            },
            InternalAction::SetInsertStyle(style) => {
                match ctx.persist.insert {
                    None => {
                        ctx.persist.insert = Some(*style);
                    },
                    Some(ref old) => {
                        if style == old {
                            ctx.persist.insert = Some(!*style);
                        } else {
                            ctx.persist.insert = Some(*style);
                        }
                    },
                }
            },
            InternalAction::SetTargetShape(f, shape) => {
                match ctx.persist.shape {
                    Some(curr) if f.matches(&curr) => {
                        ctx.persist.shape = Some(*shape);
                    },
                    None => {
                        ctx.persist.shape = Some(*shape);
                    },
                    _ => {},
                }
            },
            InternalAction::SetOperation(op) => {
                ctx.action.operation = op.clone();
            },
            InternalAction::SetTarget(et) => {
                ctx.action.target = Some(et.clone());
            },
            InternalAction::SetPostMode(ps) => {
                ctx.action.postmode = Some(*ps);
            },
        }
    }
}

#[derive(Debug)]
enum ExternalAction<P: Application> {
    Something(Action<P>),
    CountAlters(Vec<Action<P>>, Vec<Action<P>>),
    MacroToggle(bool),
    PostAction,
}

impl<P: Application> ExternalAction<P> {
    fn resolve(&self, context: &mut VimContext<P>) -> Vec<Action<P>> {
        match self {
            ExternalAction::Something(act) => vec![act.clone()],
            ExternalAction::CountAlters(acts1, acts2) => {
                if context.action.count.is_none() {
                    acts1.clone()
                } else {
                    acts2.clone()
                }
            },
            ExternalAction::PostAction => {
                if let Some(target) = context.action.target.take() {
                    vec![Action::Edit(Specifier::Contextual, target)]
                } else {
                    vec![Action::NoOp]
                }
            },
            ExternalAction::MacroToggle(reqrec) => {
                let recording = context.persist.recording.is_some();

                if *reqrec && !recording {
                    return vec![];
                } else if recording {
                    context.persist.recording = None;
                } else if let Some(reg) = context.action.register {
                    let append = context.action.register_append;
                    context.persist.recording = Some((reg, append));
                } else {
                    context.persist.recording = Some((Register::UnnamedMacro, false));
                }

                vec![MacroAction::ToggleRecording.into()]
            },
        }
    }
}

impl<P: Application> Clone for ExternalAction<P> {
    fn clone(&self) -> Self {
        match self {
            ExternalAction::Something(act) => ExternalAction::Something(act.clone()),
            ExternalAction::CountAlters(act1, act2) => {
                ExternalAction::CountAlters(act1.clone(), act2.clone())
            },
            ExternalAction::MacroToggle(reqrec) => ExternalAction::MacroToggle(*reqrec),
            ExternalAction::PostAction => ExternalAction::PostAction,
        }
    }
}

/// Description of actions to take after an input sequence.
#[derive(Debug)]
pub struct InputStep<P: Application> {
    internal: Vec<InternalAction>,
    external: Vec<ExternalAction<P>>,
    fallthrough_mode: Option<VimMode>,
    nextm: Option<VimMode>,
}

impl<P: Application> InputStep<P> {
    /// Create a new step that input keys can map to.
    pub fn new() -> Self {
        InputStep {
            internal: vec![],
            external: vec![],
            fallthrough_mode: None,
            nextm: None,
        }
    }

    /// Set the [actions](Action) that this step produces.
    pub fn actions(mut self, acts: Vec<Action<P>>) -> Self {
        self.external = acts.into_iter().map(ExternalAction::Something).collect();
        self
    }
}

impl<P: Application> Clone for InputStep<P> {
    fn clone(&self) -> Self {
        Self {
            internal: self.internal.clone(),
            external: self.external.clone(),
            fallthrough_mode: self.fallthrough_mode.clone(),
            nextm: self.nextm.clone(),
        }
    }
}

impl<P: Application> Step<TerminalKey> for InputStep<P> {
    type A = Action<P>;
    type C = VimContext<P>;
    type Class = CommonKeyClass;
    type M = VimMode;
    type Sequence = RepeatType;

    fn is_unmapped(&self) -> bool {
        match self {
            InputStep {
                internal,
                external,
                fallthrough_mode: None,
                nextm: None,
            } => internal.len() == 0 && external.len() == 0,
            _ => false,
        }
    }

    fn fallthrough(&self) -> Option<Self::M> {
        self.fallthrough_mode
    }

    fn step(&self, ctx: &mut VimContext<P>) -> (Vec<Action<P>>, Option<Self::M>) {
        match (self.nextm, self.internal.as_slice(), ctx.persist.shape) {
            (Some(VimMode::Visual), [InternalAction::SetTargetShape(f, s1)], Some(ref s2))
                if f.matches(s2) && s1 == s2 =>
            {
                return (vec![], Some(VimMode::Normal));
            },
            (Some(VimMode::Select), [InternalAction::SetTargetShape(f, s1)], Some(ref s2))
                if f.matches(s2) && s1 == s2 =>
            {
                return (vec![], Some(VimMode::Normal));
            },
            (_, internal, _) => {
                for iact in internal.iter() {
                    iact.run(ctx);
                }

                let external: Vec<Action<P>> =
                    self.external.iter().flat_map(|act| act.resolve(ctx)).collect();

                if external.len() > 0 {
                    return (external, ctx.action.postmode.take().or(self.nextm));
                } else {
                    return (external, self.nextm);
                }
            },
        }
    }
}

macro_rules! act {
    ($ext: expr) => {
        isv!(vec![], vec![ExternalAction::Something($ext)])
    };
    ($ext: expr, $ns: expr) => {
        isv!(vec![], vec![ExternalAction::Something($ext)], $ns)
    };
}

macro_rules! count_alters {
    ($act1: expr, $act2: expr) => {
        isv!(vec![], vec![ExternalAction::CountAlters(vec![$act1], vec![$act2])])
    };
    ($act1: expr, $act2: expr, $ns: expr) => {
        isv!(vec![], vec![ExternalAction::CountAlters(vec![$act1], vec![$act2])], $ns)
    };
}

macro_rules! iact {
    ($int: expr) => {
        isv!(vec![$int], vec![])
    };
    ($int: expr, $ns: expr) => {
        isv!(vec![$int], vec![], $ns)
    };
}

macro_rules! isv {
    () => {
        InputStep {
            internal: vec![],
            external: vec![],
            fallthrough_mode: None,

            nextm: None,
        }
    };
    ($ints: expr, $exts: expr) => {
        InputStep {
            internal: $ints,
            external: $exts,
            fallthrough_mode: None,

            nextm: None,
        }
    };
    ($ints: expr, $exts: expr, $ns: expr) => {
        InputStep {
            internal: $ints,
            external: $exts,
            fallthrough_mode: None,

            nextm: Some($ns),
        }
    };
}

macro_rules! is {
    ($int: expr, $ext: expr) => {
        isv!(vec![$int], vec![ExternalAction::Something($ext)])
    };
    ($int: expr, $ext: expr, $ns: expr) => {
        isv!(vec![$int], vec![ExternalAction::Something($ext)], $ns)
    };
}

macro_rules! fallthrough {
    ($mode: expr) => {
        InputStep {
            internal: vec![],
            external: vec![],
            fallthrough_mode: Some($mode),

            nextm: None,
        }
    };
    ($mode: expr, $iacts: expr) => {
        InputStep {
            internal: $iacts,
            external: vec![],
            fallthrough_mode: Some($mode),

            nextm: None,
        }
    };
}

macro_rules! shaped_filter {
    ($f: expr, $shape: expr, $act: expr) => {
        is!(InternalAction::SetTargetShape($f, $shape), $act)
    };
    ($f: expr, $shape: expr, $act: expr, $nm: expr) => {
        is!(InternalAction::SetTargetShape($f, $shape), $act, $nm)
    };
}

macro_rules! shaped {
    ($shape: expr, $act: expr) => {
        shaped_filter!(TargetShapeFilter::ALL, $shape, $act)
    };
    ($shape: expr, $act: expr, $nm: expr) => {
        shaped_filter!(TargetShapeFilter::ALL, $shape, $act, $nm)
    };
}

macro_rules! scrollcpv {
    ($p: expr, $fw: literal) => {
        if $fw {
            isv!(vec![], vec![
                ExternalAction::Something(Action::Edit(
                    Specifier::Exact(EditAction::Motion),
                    EditTarget::Motion(MoveType::FirstWord(MoveDir1D::Next), Count::Exact(0))
                )),
                ExternalAction::CountAlters(
                    vec![Action::Scroll(ScrollStyle::CursorPos($p, Axis::Vertical))],
                    vec![Action::Scroll(ScrollStyle::LinePos($p, Count::Contextual))],
                ),
            ])
        } else {
            count_alters!(
                Action::Scroll(ScrollStyle::CursorPos($p, Axis::Vertical)),
                Action::Scroll(ScrollStyle::LinePos($p, Count::Contextual))
            )
        }
    };
}

macro_rules! scrollcph {
    ($p: expr) => {
        scroll!(ScrollStyle::CursorPos($p, Axis::Horizontal))
    };
}

macro_rules! edit_target_nocount {
    ($ea: expr, $et: expr, $mode: expr) => {
        count_alters!(Action::Edit(Specifier::Exact($ea), $et), Action::NoOp, $mode)
    };
    ($ea: expr, $et: expr) => {
        count_alters!(Action::Edit(Specifier::Exact($ea), $et), Action::NoOp)
    };
}

macro_rules! edit_nocount {
    ($ea: expr, $mt: expr, $c: expr, $mode: expr) => {
        edit_target_nocount!($ea, EditTarget::Motion($mt, $c), $mode)
    };
    ($ea: expr, $mt: expr, $c: literal) => {
        edit_target_nocount!($ea, EditTarget::Motion($mt, Count::Exact($c)))
    };
    ($ea: expr, $mt: expr) => {
        edit_target_nocount!($ea, EditTarget::Motion($mt, Count::Contextual))
    };
}

macro_rules! edit_selection_nocount {
    ($ea: expr) => {
        edit_target_nocount!($ea, EditTarget::Selection, VimMode::Normal)
    };
    ($ea: expr, $mode: expr) => {
        edit_target_nocount!($ea, EditTarget::Selection, $mode)
    };
}

macro_rules! tilde {
    () => {
        isv!(vec![InternalAction::SetCursorEnd(CursorEnd::End)], vec![ExternalAction::Something(
            Action::Edit(
                Specifier::Exact(EditAction::ChangeCase(Case::Toggle)),
                EditTarget::Motion(MoveType::Column(MoveDir1D::Next, false), Count::Contextual),
            )
        )])
    };
}

macro_rules! change_target {
    ($et: expr) => {
        isv!(
            vec![InternalAction::SetInsertStyle(InsertStyle::Insert)],
            vec![ExternalAction::Something(Action::Edit(
                Specifier::Exact(EditAction::Delete),
                $et
            ))],
            VimMode::Insert
        )
    };
}

macro_rules! change_range {
    ($rt: expr) => {
        change_target!(EditTarget::Range($rt, true, Count::Contextual))
    };
}

macro_rules! change {
    ($mt: expr) => {
        change_target!(EditTarget::Motion($mt, Count::Contextual))
    };
    ($mt: expr, $c: literal) => {
        change_target!(EditTarget::Motion($mt, Count::Exact($c)))
    };
    ($mt: expr, $c: expr) => {
        change_target!(EditTarget::Motion($mt, $c))
    };
}

macro_rules! charsearch {
    ($d: expr, $i: expr) => {
        fallthrough!(VimMode::CharSearchSuffix, vec![
            InternalAction::SetSearchCharParams($d, $i),
            InternalAction::SetTarget(EditTarget::Search(
                SearchType::Char(false),
                MoveDirMod::Same,
                Count::Contextual
            ))
        ])
    };
}

macro_rules! charsearch_suffix {
    () => {
        isv!(vec![InternalAction::SetSearchChar], vec![ExternalAction::PostAction])
    };
}

macro_rules! charreplace {
    ($v: expr) => {
        fallthrough!(VimMode::CharReplaceSuffix, vec![
            InternalAction::SetOperation(EditAction::Replace($v)),
            InternalAction::SetTarget(EditTarget::Motion(
                MoveType::Column(MoveDir1D::Next, false),
                Count::Contextual
            ))
        ])
    };
    ($v: expr, $et: expr) => {
        fallthrough!(VimMode::CharReplaceSuffix, vec![
            InternalAction::SetOperation(EditAction::Replace($v)),
            InternalAction::SetTarget($et)
        ])
    };
}

macro_rules! charreplace_suffix {
    () => {
        isv!(
            vec![InternalAction::SetReplaceChar(None)],
            vec![ExternalAction::PostAction],
            VimMode::Normal
        )
    };
    ($c: expr) => {
        isv!(
            vec![InternalAction::SetReplaceChar(Some($c))],
            vec![ExternalAction::PostAction],
            VimMode::Normal
        )
    };
}

macro_rules! edit_selection {
    ($ea: expr) => {
        edit_target!($ea, EditTarget::Selection, VimMode::Normal)
    };
    ($ea: expr, $mode: expr) => {
        edit_target!($ea, EditTarget::Selection, $mode)
    };
}

macro_rules! change_selection_lines {
    () => {
        isv!(
            vec![
                InternalAction::SetTargetShape(TargetShapeFilter::ALL, TargetShape::LineWise),
                InternalAction::SetInsertStyle(InsertStyle::Insert),
            ],
            vec![ExternalAction::Something(Action::Edit(
                Specifier::Exact(EditAction::Delete),
                EditTarget::Selection
            ))],
            VimMode::Insert
        )
    };
}

macro_rules! motion {
    ($iacts: expr) => {
        fallthrough!(VimMode::OperationPending, $iacts)
    };
}

macro_rules! edit_motion {
    ($ea: expr) => {
        motion!(vec![
            InternalAction::SetOperation($ea),
            InternalAction::SetPostMode(VimMode::Normal),
        ])
    };
    ($ea: expr, $mode: expr) => {
        motion!(vec![
            InternalAction::SetOperation($ea),
            InternalAction::SetPostMode($mode),
        ])
    };
    ($ea: expr, $mode: expr, $style: expr) => {
        motion!(vec![
            InternalAction::SetOperation($ea),
            InternalAction::SetPostMode($mode),
            InternalAction::SetInsertStyle($style),
        ])
    };
}

macro_rules! edit_lines {
    ($ea: expr) => {
        edit_target!($ea, RangeType::Line.into(), VimMode::Normal)
    };
}

macro_rules! edit_target_end {
    ($et: expr) => {
        act!(Action::Edit(Specifier::Contextual, $et))
    };
}

macro_rules! edit_target_end_shaped {
    ($shape: expr, $et: expr) => {
        shaped!($shape, Action::Edit(Specifier::Contextual, $et))
    };
}

macro_rules! edit_target_end_ca {
    ($et1: expr, $et2: expr) => {
        count_alters!(
            Action::Edit(Specifier::Contextual, $et1),
            Action::Edit(Specifier::Contextual, $et2)
        )
    };
}

macro_rules! edit_range_end {
    ($rt: expr) => {
        edit_target_end!(EditTarget::Range($rt, true, Count::Contextual))
    };
    ($rt: expr, $inc: expr) => {
        edit_target_end!(EditTarget::Range($rt, $inc, Count::Contextual))
    };
    ($rt: expr, $inc: expr, $c: literal) => {
        edit_target_end!(EditTarget::Range($rt, $inc, Count::Exact($c)))
    };
    ($rt: expr, $inc: expr, $c: expr) => {
        edit_target_end!(EditTarget::Range($rt, $inc, $c))
    };
}

macro_rules! edit_search_end {
    ($st: expr, $mod: expr) => {
        edit_target_end!(EditTarget::Search($st, $mod, Count::Contextual))
    };
    ($st: expr, $mod: expr, $c: literal) => {
        edit_target_end!(EditTarget::Search($st, $mod, Count::Exact($c)))
    };
    ($st: expr, $mod: expr, $c: expr) => {
        edit_target_end!(EditTarget::Search($st, $mod, $c))
    };
}

macro_rules! edit_word_search_end {
    ($style: expr, $boundary: expr, $dir: expr) => {
        is!(
            InternalAction::SetSearchRegexParams($dir),
            Action::Edit(
                Specifier::Contextual,
                EditTarget::Search(
                    SearchType::Word($style, $boundary),
                    MoveDirMod::Same,
                    Count::Contextual
                )
            )
        )
    };
}

macro_rules! edit_end {
    ($mt: expr) => {
        edit_target_end!(EditTarget::Motion($mt, Count::Contextual))
    };
    ($mt: expr, $c: literal) => {
        edit_target_end!(EditTarget::Motion($mt, Count::Exact($c)))
    };
    ($mt: expr, $c: expr) => {
        edit_target_end!(EditTarget::Motion($mt, $c))
    };
}

macro_rules! edit_end_shaped {
    ($shape: expr, $mt: expr) => {
        edit_target_end_shaped!($shape, EditTarget::Motion($mt, Count::Contextual))
    };
    ($shape: expr, $mt: expr, $c: literal) => {
        edit_target_end_shaped!($shape, EditTarget::Motion($mt, Count::Exact($c)))
    };
    ($shape: expr, $mt: expr, $c: expr) => {
        edit_target_end_shaped!($shape, EditTarget::Motion($mt, $c))
    };
}

macro_rules! edit_end_ca {
    ($mt1: expr, $mt2: expr) => {
        edit_target_end_ca!(
            EditTarget::Motion($mt1, Count::Contextual),
            EditTarget::Motion($mt2, Count::Contextual)
        )
    };
}

macro_rules! normal {
    () => {
        goto!(VimMode::Normal)
    };
}

macro_rules! insert {
    ($style: expr) => {
        isv!(
            vec![InternalAction::SetInsertStyle($style)],
            vec![ExternalAction::Something(
                CursorAction::Split(Count::Contextual).into()
            )],
            VimMode::Insert
        )
    };
    ($style: expr, $mt: expr) => {
        isv!(
            vec![InternalAction::SetInsertStyle($style)],
            vec![
                ExternalAction::Something(Action::Edit(
                    Specifier::Exact(EditAction::Motion),
                    EditTarget::Motion($mt, Count::Exact(1))
                )),
                ExternalAction::Something(CursorAction::Split(Count::Contextual).into()),
            ],
            VimMode::Insert
        )
    };
    ($style: expr, $mt: expr, $c: expr) => {
        isv!(
            vec![InternalAction::SetInsertStyle($style)],
            vec![
                ExternalAction::Something(Action::Edit(
                    Specifier::Exact(EditAction::Motion),
                    EditTarget::Motion($mt, Count::Exact($c))
                )),
                ExternalAction::Something(CursorAction::Split(Count::Contextual).into()),
            ],
            VimMode::Insert
        )
    };
}

macro_rules! paste_register {
    ($dir: expr, $reg: expr) => {
        is!(
            InternalAction::SetRegister($reg),
            Action::InsertText(InsertTextAction::Paste($dir, Count::Contextual))
        )
    };
    ($dir: expr, $reg: expr, $nm: expr) => {
        is!(
            InternalAction::SetRegister($reg),
            Action::InsertText(InsertTextAction::Paste($dir, Count::Contextual)),
            $nm
        )
    };
}

macro_rules! open_lines {
    ($dir: expr) => {
        isv!(
            vec![InternalAction::SetInsertStyle(InsertStyle::Insert)],
            vec![
                ExternalAction::Something(CursorAction::Split(Count::Contextual).into()),
                ExternalAction::Something(
                    InsertTextAction::OpenLine(TargetShape::LineWise, $dir, 1.into()).into()
                )
            ],
            VimMode::Insert
        )
    };
}

macro_rules! edit_selection_nochar {
    ($ea: expr) => {
        shaped_filter!(
            TargetShapeFilter::CHAR,
            TargetShape::LineWise,
            Action::Edit(Specifier::Exact($ea), EditTarget::Selection),
            VimMode::Normal
        )
    };
}

macro_rules! delete_selection_nochar {
    ($cursor: expr, $et: expr) => {
        isv!(
            vec![InternalAction::SetTargetShape(
                TargetShapeFilter::CHAR,
                TargetShape::LineWise
            )],
            vec![
                ExternalAction::Something(
                    SelectionAction::Split(SelectionSplitStyle::Lines, TargetShapeFilter::ALL)
                        .into()
                ),
                ExternalAction::Something(SelectionAction::CursorSet($cursor).into()),
                ExternalAction::Something(Action::Edit(EditAction::Delete.into(), $et)),
            ],
            VimMode::Normal
        )
    };
}

macro_rules! change_selection_nochar {
    ($cursor: expr, $et: expr) => {
        isv!(
            vec![
                InternalAction::SetTargetShape(TargetShapeFilter::CHAR, TargetShape::LineWise),
                InternalAction::SetInsertStyle(InsertStyle::Insert),
            ],
            vec![
                ExternalAction::Something(
                    SelectionAction::Split(SelectionSplitStyle::Lines, TargetShapeFilter::ALL)
                        .into()
                ),
                ExternalAction::Something(SelectionAction::CursorSet($cursor).into()),
                ExternalAction::Something(Action::Edit(EditAction::Delete.into(), $et)),
                ExternalAction::Something(CursorAction::Split(Count::Contextual).into()),
            ],
            VimMode::Insert
        )
    };
}

macro_rules! insert_visual {
    ($cursor: expr) => {
        isv!(
            vec![InternalAction::SetInsertStyle(InsertStyle::Insert)],
            vec![
                ExternalAction::Something(
                    SelectionAction::Split(SelectionSplitStyle::Lines, TargetShapeFilter::BLOCK)
                        .into()
                ),
                ExternalAction::Something(SelectionAction::CursorSet($cursor).into()),
                ExternalAction::Something(CursorAction::Split(Count::Contextual).into()),
            ],
            VimMode::Insert
        )
    };
    ($cursor: expr, $et: expr) => {
        isv!(
            vec![InternalAction::SetInsertStyle(InsertStyle::Insert)],
            vec![
                ExternalAction::Something(
                    SelectionAction::Split(SelectionSplitStyle::Lines, TargetShapeFilter::BLOCK)
                        .into()
                ),
                ExternalAction::Something(SelectionAction::CursorSet($cursor).into()),
                ExternalAction::Something(Action::Edit(EditAction::Delete.into(), $et)),
                ExternalAction::Something(CursorAction::Split(Count::Contextual).into()),
            ],
            VimMode::Insert
        )
    };
}

macro_rules! shape {
    ($shape: expr) => {
        iact!(InternalAction::SetTargetShape(TargetShapeFilter::ALL, $shape))
    };
    ($shape: expr, $mode: expr) => {
        iact!(InternalAction::SetTargetShape(TargetShapeFilter::ALL, $shape), $mode)
    };
}

macro_rules! start_selection {
    ($shape: expr, $mode: expr) => {
        shaped!(
            $shape,
            Action::Edit(Specifier::Exact(EditAction::Motion), EditTarget::CurrentPosition),
            $mode
        )
    };
}

macro_rules! selection_resize_search {
    ($style: expr, $dir: expr) => {
        shaped!(
            TargetShape::CharWise,
            Action::Selection(SelectionAction::Resize(
                $style,
                EditTarget::Search(SearchType::Regex, MoveDirMod::Exact($dir), Count::Contextual)
            )),
            VimMode::Visual
        )
    };
}

macro_rules! visual {
    ($shape: expr) => {
        start_selection!($shape, VimMode::Visual)
    };
}

macro_rules! select {
    ($shape: expr) => {
        start_selection!($shape, VimMode::Select)
    };
}

macro_rules! tab {
    ($act: expr) => {
        act!(Action::Tab($act), VimMode::Normal)
    };
    ($act1: expr, $act2: expr) => {
        count_alters!(Action::Tab($act1), Action::Tab($act2), VimMode::Normal)
    };
}

macro_rules! tab_focus {
    ($fc: expr) => {
        tab!(TabAction::Focus($fc))
    };
    ($fc1: expr, $fc2: expr) => {
        tab!(TabAction::Focus($fc1), TabAction::Focus($fc2))
    };
}

macro_rules! window {
    ($act: expr) => {
        act!(Action::Window($act), VimMode::Normal)
    };
    ($act1: expr, $act2: expr) => {
        count_alters!(Action::Window($act1), Action::Window($act2), VimMode::Normal)
    };
}

macro_rules! window_resize {
    ($axis: expr, $change: expr) => {
        window!(WindowAction::Resize($axis, $change))
    };
}

macro_rules! window_clear_size {
    () => {
        window!(WindowAction::ClearSizes)
    };
}

macro_rules! window_exchange {
    ($fc: expr) => {
        window!(WindowAction::Exchange($fc))
    };
    ($fc1: expr, $fc2: expr) => {
        window!(WindowAction::Exchange($fc1), WindowAction::Exchange($fc2))
    };
}

macro_rules! window_focus {
    ($fc: expr) => {
        window!(WindowAction::Focus($fc))
    };
    ($fc1: expr, $fc2: expr) => {
        window!(WindowAction::Focus($fc1), WindowAction::Focus($fc2))
    };
}

macro_rules! window_close_one {
    ($style: expr, $fc: expr, $flags: expr) => {
        window!(WindowAction::Close($style($fc), $flags))
    };
}

macro_rules! window_close {
    ($style: expr, $f1: expr, $f2: expr) => {
        window!(
            WindowAction::Close($style($f1), CloseFlags::NONE),
            WindowAction::Close($style($f2), CloseFlags::NONE)
        )
    };
    ($style: expr, $f1: expr, $f2: expr, $flags: expr) => {
        window!(WindowAction::Close($style($f1), $flags), WindowAction::Close($style($f2), $flags))
    };
}

macro_rules! window_quit {
    ($style: expr, $f1: expr, $f2: expr) => {
        window_close!($style, $f1, $f2, CloseFlags::QUIT)
    };
}

macro_rules! window_split {
    ($axis: expr) => {
        isv!(
            vec![],
            vec![ExternalAction::CountAlters(
                vec![WindowAction::Split($axis, MoveDir1D::Previous, Count::Contextual).into()],
                vec![
                    WindowAction::Split($axis, MoveDir1D::Previous, Count::Exact(1)).into(),
                    WindowAction::Resize($axis, SizeChange::Exact(Count::Contextual)).into(),
                ],
            )],
            VimMode::Normal
        )
    };
}

macro_rules! cmdbar_focus {
    ($type: expr) => {
        cmdbar!(CommandBarAction::Focus($type), VimMode::Command)
    };
}

macro_rules! search {
    ($dir: expr) => {
        is!(
            InternalAction::SetSearchRegexParams($dir),
            Action::CommandBar(CommandBarAction::Focus(CommandType::Search($dir, false))),
            VimMode::Command
        )
    };
}

macro_rules! command_unfocus {
    () => {
        cmdbar!(CommandBarAction::Abort, VimMode::Normal)
    };
}

#[rustfmt::skip]
fn default_keys<P: Application>() -> Vec<(MappedModes, &'static str, InputStep<P>)> {
    [
        // Normal, Visual, Select, Insert mode keys
        ( NVIMAP, "<C-\\><C-N>", normal!() ),
        ( NVIMAP, "<C-End>", edit_target_end!(EditTarget::Boundary(RangeType::Buffer, true, MoveTerminus::End, Count::Contextual)) ),

        // Normal, Visual, Select, Operation Pending mode keys
        ( MAP, "<C-H>", edit_end!(MoveType::Column(MoveDir1D::Previous, true)) ),
        ( MAP, "<C-J>", edit_end!(MoveType::Line(MoveDir1D::Next)) ),
        ( MAP, "<C-N>", edit_end!(MoveType::Line(MoveDir1D::Next)) ),
        ( MAP, "<C-P>", edit_end!(MoveType::Line(MoveDir1D::Previous)) ),
        ( MAP, "<Up>", edit_end!(MoveType::Line(MoveDir1D::Previous)) ),
        ( MAP, "<Down>", edit_end!(MoveType::Line(MoveDir1D::Next)) ),
        ( MAP, "<Left>", edit_end!(MoveType::Column(MoveDir1D::Previous, false)) ),
        ( MAP, "<Right>", edit_end!(MoveType::Column(MoveDir1D::Next, false)) ),
        ( MAP, "<S-Left>", edit_end!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Previous)) ),
        ( MAP, "<S-Right>", edit_end!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Next)) ),
        ( MAP, "<C-Left>", edit_end!(MoveType::WordBegin(WordStyle::Big, MoveDir1D::Previous)) ),
        ( MAP, "<C-Right>", edit_end!(MoveType::WordBegin(WordStyle::Big, MoveDir1D::Next)) ),
        ( MAP, "<BS>", edit_end!(MoveType::Column(MoveDir1D::Previous, true)) ),
        ( MAP, "<End>", edit_end!(MoveType::LinePos(MovePosition::End), Count::MinusOne) ),
        ( MAP, "<Home>", edit_end!(MoveType::LinePos(MovePosition::Beginning), 0) ),

        // Normal, Visual, Operation Pending mode keys
        ( NXOMAP, "0", edit_end!(MoveType::LinePos(MovePosition::Beginning), 0) ),
        ( NXOMAP, "b", edit_end!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Previous)) ),
        ( NXOMAP, "B", edit_end!(MoveType::WordBegin(WordStyle::Big, MoveDir1D::Previous)) ),
        ( NXOMAP, "e", edit_end!(MoveType::WordEnd(WordStyle::Little, MoveDir1D::Next)) ),
        ( NXOMAP, "E", edit_end!(MoveType::WordEnd(WordStyle::Big, MoveDir1D::Next)) ),
        ( NXOMAP, "f", charsearch!(MoveDir1D::Next, true) ),
        ( NXOMAP, "F", charsearch!(MoveDir1D::Previous, true) ),
        ( NXOMAP, "g0", edit_end!(MoveType::ScreenLinePos(MovePosition::Beginning), 0) ),
        ( NXOMAP, "ge", edit_end!(MoveType::WordEnd(WordStyle::Little, MoveDir1D::Previous)) ),
        ( NXOMAP, "gE", edit_end!(MoveType::WordEnd(WordStyle::Big, MoveDir1D::Previous)) ),
        ( NXOMAP, "gg", edit_end_ca!(MoveType::BufferPos(MovePosition::Beginning), MoveType::BufferLineOffset) ),
        ( NXOMAP, "gj", edit_end!(MoveType::ScreenLine(MoveDir1D::Next)) ),
        ( NXOMAP, "gk", edit_end!(MoveType::ScreenLine(MoveDir1D::Previous)) ),
        ( NXOMAP, "gm", edit_end!(MoveType::ScreenLinePos(MovePosition::Middle), 0) ),
        ( NXOMAP, "gM", edit_target_end_ca!(EditTarget::Motion(MoveType::LinePos(MovePosition::Middle), Count::MinusOne), EditTarget::Motion(MoveType::LinePercent, Count::Contextual)) ),
        ( NXOMAP, "go", edit_end!(MoveType::BufferByteOffset) ),
        ( NXOMAP, "g_", edit_end!(MoveType::FinalNonBlank(MoveDir1D::Next), Count::MinusOne) ),
        ( NXOMAP, "g^", edit_end!(MoveType::ScreenFirstWord(MoveDir1D::Next), 0) ),
        ( NXOMAP, "g$", edit_end!(MoveType::ScreenLinePos(MovePosition::End), Count::MinusOne) ),
        ( NXOMAP, "g#", edit_word_search_end!(WordStyle::Little, false, MoveDir1D::Previous) ),
        ( NXOMAP, "g*", edit_word_search_end!(WordStyle::Little, false, MoveDir1D::Next) ),
        ( NXOMAP, "g'{mark}", unmapped!() ),
        ( NXOMAP, "g`{mark}", unmapped!() ),
        ( NXOMAP, "g<Down>", edit_end!(MoveType::ScreenLine(MoveDir1D::Next)) ),
        ( NXOMAP, "g<Up>", edit_end!(MoveType::ScreenLine(MoveDir1D::Previous)) ),
        ( NXOMAP, "g<End>", edit_end!(MoveType::ScreenLinePos(MovePosition::End), Count::MinusOne) ),
        ( NXOMAP, "g<Home>", edit_end!(MoveType::ScreenLinePos(MovePosition::Beginning), 1) ),
        ( NXOMAP, "G", edit_end_ca!(MoveType::BufferPos(MovePosition::End), MoveType::BufferLineOffset) ),
        ( NXOMAP, "h", edit_end!(MoveType::Column(MoveDir1D::Previous, false)) ),
        ( NXOMAP, "H", edit_end!(MoveType::ViewportPos(MovePosition::Beginning)) ),
        ( NXOMAP, "j", edit_end!(MoveType::Line(MoveDir1D::Next)) ),
        ( NXOMAP, "k", edit_end!(MoveType::Line(MoveDir1D::Previous)) ),
        ( NXOMAP, "l", edit_end!(MoveType::Column(MoveDir1D::Next, false)) ),
        ( NXOMAP, "L", edit_end!(MoveType::ViewportPos(MovePosition::End)) ),
        ( NXOMAP, "M", edit_end!(MoveType::ViewportPos(MovePosition::Middle)) ),
        ( NXOMAP, "t", charsearch!(MoveDir1D::Next, false) ),
        ( NXOMAP, "T", charsearch!(MoveDir1D::Previous, false) ),
        ( NXOMAP, "w", edit_end!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Next)) ),
        ( NXOMAP, "W", edit_end!(MoveType::WordBegin(WordStyle::Big, MoveDir1D::Next)) ),
        ( NXOMAP, "`{mark}", edit_target_end!(EditTarget::CharJump(Specifier::Contextual)) ),
        ( NXOMAP, "'{mark}", edit_target_end!(EditTarget::LineJump(Specifier::Contextual)) ),
        ( NXOMAP, " ", edit_end!(MoveType::Column(MoveDir1D::Next, true)) ),
        ( NXOMAP, "^", edit_end_shaped!(TargetShape::CharWise, MoveType::FirstWord(MoveDir1D::Next), 0) ),
        ( NXOMAP, "$", edit_end!(MoveType::LinePos(MovePosition::End), Count::MinusOne) ),
        ( NXOMAP, "_", edit_end!(MoveType::FirstWord(MoveDir1D::Next), Count::MinusOne) ),
        ( NXOMAP, "-", edit_end!(MoveType::FirstWord(MoveDir1D::Previous)) ),
        ( NXOMAP, "+", edit_end!(MoveType::FirstWord(MoveDir1D::Next)) ),
        ( NXOMAP, "%", edit_end_ca!(MoveType::ItemMatch, MoveType::BufferLinePercent) ),
        ( NXOMAP, "#", edit_word_search_end!(WordStyle::Little, true, MoveDir1D::Previous) ),
        ( NXOMAP, "*", edit_word_search_end!(WordStyle::Little, true, MoveDir1D::Next) ),
        ( NXOMAP, "?", search!(MoveDir1D::Previous) ),
        ( NXOMAP, "/", search!(MoveDir1D::Next) ),
        ( NXOMAP, "|", edit_end!(MoveType::LineColumnOffset) ),
        ( NXOMAP, ";", edit_search_end!(SearchType::Char(false), MoveDirMod::Same) ),
        ( NXOMAP, ",", edit_search_end!(SearchType::Char(false), MoveDirMod::Flip) ),
        ( NXOMAP, "(", edit_end!(MoveType::SentenceBegin(MoveDir1D::Previous)) ),
        ( NXOMAP, ")", edit_end!(MoveType::SentenceBegin(MoveDir1D::Next)) ),
        ( NXOMAP, "{", edit_end!(MoveType::ParagraphBegin(MoveDir1D::Previous)) ),
        ( NXOMAP, "}", edit_end!(MoveType::ParagraphBegin(MoveDir1D::Next)) ),
        ( NXOMAP, "[[", edit_end!(MoveType::SectionBegin(MoveDir1D::Previous)) ),
        ( NXOMAP, "[]", edit_end!(MoveType::SectionEnd(MoveDir1D::Previous)) ),
        ( NXOMAP, "][", edit_end!(MoveType::SectionEnd(MoveDir1D::Next)) ),
        ( NXOMAP, "]]", edit_end!(MoveType::SectionBegin(MoveDir1D::Next)) ),
        ( NXOMAP, "[(", unmapped!() ),
        ( NXOMAP, "[{", unmapped!() ),
        ( NXOMAP, "])", unmapped!() ),
        ( NXOMAP, "]}", unmapped!() ),
        ( NXOMAP, "]'", unmapped!() ),
        ( NXOMAP, "]`", unmapped!() ),
        ( NXOMAP, "['", unmapped!() ),
        ( NXOMAP, "[`", unmapped!() ),

        // Normal, Visual, Select mode keys
        ( NVMAP, "<C-B>", scroll2d!(MoveDir2D::Up, ScrollSize::Page) ),
        ( NVMAP, "<C-D>", scroll2d!(MoveDir2D::Down, ScrollSize::HalfPage) ),
        ( NVMAP, "<C-E>", scroll2d!(MoveDir2D::Down, ScrollSize::Cell) ),
        ( NVMAP, "<C-F>", scroll2d!(MoveDir2D::Down, ScrollSize::Page) ),
        ( NVMAP, "<C-U>", scroll2d!(MoveDir2D::Up, ScrollSize::HalfPage) ),
        ( NVMAP, "<C-V>", visual!(TargetShape::BlockWise) ),
        ( NVMAP, "<C-W>b", window_focus!(FocusChange::Position(MovePosition::End)) ),
        ( NVMAP, "<C-W>c", window_close!(CloseTarget::Single, FocusChange::Current, FocusChange::Offset(Count::Contextual, true)) ),
        ( NVMAP, "<C-W>g<Tab>", tab_focus!(FocusChange::PreviouslyFocused) ),
        ( NVMAP, "<C-W>h", window_focus!(FocusChange::Direction2D(MoveDir2D::Left, Count::Contextual)) ),
        ( NVMAP, "<C-W>H", window!(WindowAction::MoveSide(MoveDir2D::Left)) ),
        ( NVMAP, "<C-W>j", window_focus!(FocusChange::Direction2D(MoveDir2D::Down, Count::Contextual)) ),
        ( NVMAP, "<C-W>J", window!(WindowAction::MoveSide(MoveDir2D::Down)) ),
        ( NVMAP, "<C-W>k", window_focus!(FocusChange::Direction2D(MoveDir2D::Up, Count::Contextual)) ),
        ( NVMAP, "<C-W>K", window!(WindowAction::MoveSide(MoveDir2D::Up)) ),
        ( NVMAP, "<C-W>l", window_focus!(FocusChange::Direction2D(MoveDir2D::Right, Count::Contextual)) ),
        ( NVMAP, "<C-W>L", window!(WindowAction::MoveSide(MoveDir2D::Right)) ),
        ( NVMAP, "<C-W>o", window_quit!(CloseTarget::AllBut, FocusChange::Current, FocusChange::Offset(Count::Contextual, true)) ),
        ( NVMAP, "<C-W>p", window_focus!(FocusChange::PreviouslyFocused) ),
        ( NVMAP, "<C-W>q", window_quit!(CloseTarget::Single, FocusChange::Current, FocusChange::Offset(Count::Contextual, true)) ),
        ( NVMAP, "<C-W>r", window!(WindowAction::Rotate(MoveDir1D::Next)) ),
        ( NVMAP, "<C-W>R", window!(WindowAction::Rotate(MoveDir1D::Previous)) ),
        ( NVMAP, "<C-W>s", window_split!(Axis::Horizontal) ),
        ( NVMAP, "<C-W>S", window_split!(Axis::Horizontal) ),
        ( NVMAP, "<C-W>t", window_focus!(FocusChange::Position(MovePosition::Beginning)) ),
        ( NVMAP, "<C-W>T", tab!(TabAction::Extract(FocusChange::Current, MoveDir1D::Next), TabAction::Extract(FocusChange::Offset(Count::Contextual, false), MoveDir1D::Previous)) ),
        ( NVMAP, "<C-W>v", window_split!(Axis::Vertical) ),
        ( NVMAP, "<C-W>w", window_focus!(FocusChange::Direction1D(MoveDir1D::Next, Count::Exact(1), true), FocusChange::Offset(Count::Contextual, true)) ),
        ( NVMAP, "<C-W>W", window_focus!(FocusChange::Direction1D(MoveDir1D::Previous, Count::Exact(1), true), FocusChange::Offset(Count::Contextual, true)) ),
        ( NVMAP, "<C-W>x", window_exchange!(FocusChange::Direction1D(MoveDir1D::Next, Count::Exact(1), false), FocusChange::Offset(Count::Contextual, false)) ),
        ( NVMAP, "<C-W>=", window_clear_size!() ),
        ( NVMAP, "<C-W>-", window_resize!(Axis::Horizontal, SizeChange::Decrease(Count::Contextual)) ),
        ( NVMAP, "<C-W>+", window_resize!(Axis::Horizontal, SizeChange::Increase(Count::Contextual)) ),
        ( NVMAP, "<C-W>_", window_resize!(Axis::Horizontal, SizeChange::Exact(Count::Contextual)) ),
        ( NVMAP, "<C-W><", window_resize!(Axis::Vertical, SizeChange::Decrease(Count::Contextual)) ),
        ( NVMAP, "<C-W>>", window_resize!(Axis::Vertical, SizeChange::Increase(Count::Contextual)) ),
        ( NVMAP, "<C-W>|", window_resize!(Axis::Vertical, SizeChange::Exact(Count::Contextual)) ),
        ( NVMAP, "<C-W><C-B>", window_focus!(FocusChange::Position(MovePosition::End)) ),
        ( NVMAP, "<C-W><C-C>", normal!() ),
        ( NVMAP, "<C-W><C-H>", window_focus!(FocusChange::Direction2D(MoveDir2D::Left, Count::Contextual)) ),
        ( NVMAP, "<C-W><C-J>", window_focus!(FocusChange::Direction2D(MoveDir2D::Down, Count::Contextual)) ),
        ( NVMAP, "<C-W><C-K>", window_focus!(FocusChange::Direction2D(MoveDir2D::Up, Count::Contextual)) ),
        ( NVMAP, "<C-W><C-L>", window_focus!(FocusChange::Direction2D(MoveDir2D::Right, Count::Contextual)) ),
        ( NVMAP, "<C-W><C-O>", window_quit!(CloseTarget::AllBut, FocusChange::Current, FocusChange::Offset(Count::Contextual, true)) ),
        ( NVMAP, "<C-W><C-Q>", window_quit!(CloseTarget::Single, FocusChange::Current, FocusChange::Offset(Count::Contextual, true)) ),
        ( NVMAP, "<C-W><C-R>", window!(WindowAction::Rotate(MoveDir1D::Next)) ),
        ( NVMAP, "<C-W><C-S>", window_split!(Axis::Horizontal) ),
        ( NVMAP, "<C-W><C-T>", window_focus!(FocusChange::Position(MovePosition::Beginning)) ),
        ( NVMAP, "<C-W><C-V>", window_split!(Axis::Vertical) ),
        ( NVMAP, "<C-W><C-W>", window_focus!(FocusChange::Direction1D(MoveDir1D::Next, Count::Exact(1), true), FocusChange::Offset(Count::Contextual, true)) ),
        ( NVMAP, "<C-W><C-X>", window_exchange!(FocusChange::Direction1D(MoveDir1D::Next, Count::Exact(1), false), FocusChange::Offset(Count::Contextual, false)) ),
        ( NVMAP, "<C-W><Left>", window_focus!(FocusChange::Direction2D(MoveDir2D::Left, Count::Contextual)) ),
        ( NVMAP, "<C-W><Down>", window_focus!(FocusChange::Direction2D(MoveDir2D::Down, Count::Contextual)) ),
        ( NVMAP, "<C-W><Up>", window_focus!(FocusChange::Direction2D(MoveDir2D::Up, Count::Contextual)) ),
        ( NVMAP, "<C-W><Right>", window_focus!(FocusChange::Direction2D(MoveDir2D::Right, Count::Contextual)) ),
        ( NVMAP, "<C-Y>", scroll2d!(MoveDir2D::Up, ScrollSize::Cell) ),
        ( NVMAP, "<S-Up>", scroll2d!(MoveDir2D::Up, ScrollSize::Page) ),
        ( NVMAP, "<S-Down>", scroll2d!(MoveDir2D::Down, ScrollSize::Page) ),
        ( NVMAP, "<PageDown>", scroll2d!(MoveDir2D::Down, ScrollSize::Page) ),
        ( NVMAP, "<PageUp>", scroll2d!(MoveDir2D::Up, ScrollSize::Page) ),

        // Normal, Visual mode keys
        ( NXMAP, "gh", select!(TargetShape::CharWise) ),
        ( NXMAP, "gH", select!(TargetShape::LineWise) ),
        ( NXMAP, "gt", tab_focus!(FocusChange::Direction1D(MoveDir1D::Next, Count::Exact(1), true), FocusChange::Offset(Count::Contextual, false)) ),
        ( NXMAP, "gT", tab_focus!(FocusChange::Direction1D(MoveDir1D::Previous, Count::Contextual, true)) ),
        ( NXMAP, "g<C-H>", select!(TargetShape::BlockWise) ),
        ( NXMAP, "m{mark}", act!(Action::Mark(Specifier::Contextual)) ),
        ( NXMAP, "q{register}", isv!(vec![], vec![ExternalAction::MacroToggle(false)]) ),
        ( NXMAP, "q", isv!(vec![], vec![ExternalAction::MacroToggle(true)]) ),
        ( NXMAP, "v", visual!(TargetShape::CharWise) ),
        ( NXMAP, "V", visual!(TargetShape::LineWise) ),
        ( NXMAP, "zb", scrollcpv!(MovePosition::End, false) ),
        ( NXMAP, "ze", scrollcph!(MovePosition::End) ),
        ( NXMAP, "zg", unmapped!() ),
        ( NXMAP, "zG", unmapped!() ),
        ( NXMAP, "zh", scroll2d!(MoveDir2D::Left, ScrollSize::Cell) ),
        ( NXMAP, "zH", scroll2d!(MoveDir2D::Left, ScrollSize::HalfPage) ),
        ( NXMAP, "zl", scroll2d!(MoveDir2D::Right, ScrollSize::Cell) ),
        ( NXMAP, "zL", scroll2d!(MoveDir2D::Right, ScrollSize::HalfPage) ),
        ( NXMAP, "zp", unmapped!() ),
        ( NXMAP, "zP", unmapped!() ),
        ( NXMAP, "zs", scrollcph!(MovePosition::Beginning) ),
        ( NXMAP, "zt", scrollcpv!(MovePosition::Beginning, false) ),
        ( NXMAP, "zug", unmapped!() ),
        ( NXMAP, "zuG", unmapped!() ),
        ( NXMAP, "zuw", unmapped!() ),
        ( NXMAP, "zuW", unmapped!() ),
        ( NXMAP, "zw", unmapped!() ),
        ( NXMAP, "zW", unmapped!() ),
        ( NXMAP, "zy", unmapped!() ),
        ( NXMAP, "zz", scrollcpv!(MovePosition::Middle, false) ),
        ( NXMAP, "z+", unmapped!() ),
        ( NXMAP, "z-", scrollcpv!(MovePosition::End, true) ),
        ( NXMAP, "z.", scrollcpv!(MovePosition::Middle, true) ),
        ( NXMAP, "z=", unmapped!() ),
        ( NXMAP, "z^", unmapped!() ),
        ( NXMAP, "z<Left>", scroll2d!(MoveDir2D::Left, ScrollSize::Cell) ),
        ( NXMAP, "z<Right>", scroll2d!(MoveDir2D::Right, ScrollSize::Cell) ),
        ( NXMAP, "z<Enter>", scrollcpv!(MovePosition::Beginning, true) ),
        ( NXMAP, "z{count}<Enter>", window_resize!(Axis::Horizontal, SizeChange::Exact(Count::Contextual)) ),

        // Visual, Operator Pending mode keys
        ( VOMAP, "aw", edit_range_end!(RangeType::Word(WordStyle::Little)) ),
        ( VOMAP, "iw", edit_range_end!(RangeType::Word(WordStyle::Little)) ),
        ( VOMAP, "aW", edit_range_end!(RangeType::Word(WordStyle::Big)) ),
        ( VOMAP, "iW", edit_range_end!(RangeType::Word(WordStyle::Big)) ),
        ( VOMAP, "as", edit_range_end!(RangeType::Sentence) ),
        ( VOMAP, "is", edit_range_end!(RangeType::Sentence) ),
        ( VOMAP, "ap", edit_range_end!(RangeType::Paragraph) ),
        ( VOMAP, "ip", edit_range_end!(RangeType::Paragraph) ),
        ( VOMAP, "a]", edit_range_end!(RangeType::Bracketed('[', ']'), true) ),
        ( VOMAP, "a[", edit_range_end!(RangeType::Bracketed('[', ']'), true) ),
        ( VOMAP, "i]", edit_range_end!(RangeType::Bracketed('[', ']'), false) ),
        ( VOMAP, "i[", edit_range_end!(RangeType::Bracketed('[', ']'), false) ),
        ( VOMAP, "a)", edit_range_end!(RangeType::Bracketed('(', ')'), true) ),
        ( VOMAP, "a(", edit_range_end!(RangeType::Bracketed('(', ')'), true) ),
        ( VOMAP, "ab", edit_range_end!(RangeType::Bracketed('(', ')'), true) ),
        ( VOMAP, "i)", edit_range_end!(RangeType::Bracketed('(', ')'), false) ),
        ( VOMAP, "i(", edit_range_end!(RangeType::Bracketed('(', ')'), false) ),
        ( VOMAP, "ib", edit_range_end!(RangeType::Bracketed('(', ')'), false) ),
        ( VOMAP, "a>", edit_range_end!(RangeType::Bracketed('<', '>'), true) ),
        ( VOMAP, "a<", edit_range_end!(RangeType::Bracketed('<', '>'), true) ),
        ( VOMAP, "i>", edit_range_end!(RangeType::Bracketed('<', '>'), false) ),
        ( VOMAP, "i<", edit_range_end!(RangeType::Bracketed('<', '>'), false) ),
        ( VOMAP, "at", edit_range_end!(RangeType::XmlTag, true) ),
        ( VOMAP, "it", edit_range_end!(RangeType::XmlTag, false) ),
        ( VOMAP, "a}", edit_range_end!(RangeType::Bracketed('{', '}'), true) ),
        ( VOMAP, "a{", edit_range_end!(RangeType::Bracketed('{', '}'), true) ),
        ( VOMAP, "aB", edit_range_end!(RangeType::Bracketed('{', '}'), true) ),
        ( VOMAP, "i}", edit_range_end!(RangeType::Bracketed('{', '}'), false) ),
        ( VOMAP, "i{", edit_range_end!(RangeType::Bracketed('{', '}'), false) ),
        ( VOMAP, "iB", edit_range_end!(RangeType::Bracketed('{', '}'), false) ),
        ( VOMAP, "a\"", edit_range_end!(RangeType::Quote('\"'), true) ),
        ( VOMAP, "i\"", edit_range_end!(RangeType::Quote('\"'), false) ),
        ( VOMAP, "a\'", edit_range_end!(RangeType::Quote('\''), true) ),
        ( VOMAP, "i\'", edit_range_end!(RangeType::Quote('\''), false) ),
        ( VOMAP, "a`", edit_range_end!(RangeType::Quote('`'), true) ),
        ( VOMAP, "i`", edit_range_end!(RangeType::Quote('`'), false) ),

        // Normal mode keys
        ( NMAP, "a", insert!(InsertStyle::Insert, MoveType::Column(MoveDir1D::Next, false)) ),
        ( NMAP, "A", insert!(InsertStyle::Insert, MoveType::LinePos(MovePosition::End), 0) ),
        ( NMAP, "c", edit_motion!(EditAction::Delete, VimMode::Insert, InsertStyle::Insert) ),
        ( NMAP, "cc", change_range!(RangeType::Line) ),
        ( NMAP, "cw", edit_end!(MoveType::WordEnd(WordStyle::Little, MoveDir1D::Next)) ),
        ( NMAP, "cW", edit_end!(MoveType::WordEnd(WordStyle::Big, MoveDir1D::Next)) ),
        ( NMAP, "C", change!(MoveType::LinePos(MovePosition::End)) ),
        ( NMAP, "d", edit_motion!(EditAction::Delete) ),
        ( NMAP, "dd", edit_lines!(EditAction::Delete) ),
        ( NMAP, "D", edit!(EditAction::Delete, MoveType::LinePos(MovePosition::End)) ),
        ( NMAP, "ga", unmapped!() ),
        ( NMAP, "gi", unmapped!() ),
        ( NMAP, "gI", insert!(InsertStyle::Insert, MoveType::LinePos(MovePosition::Beginning), 0) ),
        ( NMAP, "gJ", edit_lines!(EditAction::Join(JoinStyle::NoChange)) ),
        ( NMAP, "gn", selection_resize_search!(SelectionResizeStyle::Object, MoveDir1D::Next) ),
        ( NMAP, "gN", selection_resize_search!(SelectionResizeStyle::Object, MoveDir1D::Previous) ),
        ( NMAP, "gq", edit_motion!(EditAction::Format) ),
        ( NMAP, "gqgq", edit_lines!(EditAction::Format) ),
        ( NMAP, "gqq", edit_lines!(EditAction::Format) ),
        ( NMAP, "gr", charreplace!(true) ),
        ( NMAP, "gR", unmapped!() ),
        ( NMAP, "gu", edit_motion!(EditAction::ChangeCase(Case::Lower)) ),
        ( NMAP, "gugu", edit_lines!(EditAction::ChangeCase(Case::Lower)) ),
        ( NMAP, "guu", edit_lines!(EditAction::ChangeCase(Case::Lower)) ),
        ( NMAP, "gU", edit_motion!(EditAction::ChangeCase(Case::Upper)) ),
        ( NMAP, "gUgU", edit_lines!(EditAction::ChangeCase(Case::Upper)) ),
        ( NMAP, "gUU", edit_lines!(EditAction::ChangeCase(Case::Upper)) ),
        ( NMAP, "gv", unmapped!() ),
        ( NMAP, "gw", edit_motion!(EditAction::Format) ),
        ( NMAP, "gwgw", edit_lines!(EditAction::Format) ),
        ( NMAP, "gww", edit_lines!(EditAction::Format) ),
        ( NMAP, "g8", unmapped!() ),
        ( NMAP, "g&", unmapped!() ),
        ( NMAP, "g~", edit_motion!(EditAction::ChangeCase(Case::Toggle)) ),
        ( NMAP, "g~~", edit_lines!(EditAction::ChangeCase(Case::Toggle)) ),
        ( NMAP, "g,", jump!(PositionList::ChangeList, MoveDir1D::Next) ),
        ( NMAP, "g;", jump!(PositionList::ChangeList, MoveDir1D::Previous) ),
        ( NMAP, "g<Tab>", tab_focus!(FocusChange::PreviouslyFocused) ),
        ( NMAP, "i", insert!(InsertStyle::Insert) ),
        ( NMAP, "I", insert!(InsertStyle::Insert, MoveType::FirstWord(MoveDir1D::Next), 0) ),
        ( NMAP, "J", edit_lines!(EditAction::Join(JoinStyle::OneSpace)) ),
        ( NMAP, "K", act!(Action::KeywordLookup) ),
        ( NMAP, "o", open_lines!(MoveDir1D::Next) ),
        ( NMAP, "O", open_lines!(MoveDir1D::Previous) ),
        ( NMAP, "p", paste!(MoveDir1D::Next) ),
        ( NMAP, "P", paste!(MoveDir1D::Previous) ),
        ( NMAP, "Q", unmapped!() ),
        ( NMAP, "r", charreplace!(false) ),
        ( NMAP, "R", insert!(InsertStyle::Replace) ),
        ( NMAP, "s", change!(MoveType::Column(MoveDir1D::Next, false)) ),
        ( NMAP, "S", change_range!(RangeType::Line) ),
        ( NMAP, "u", history!(HistoryAction::Undo(Count::Contextual)) ),
        ( NMAP, "x", edit!(EditAction::Delete, MoveType::Column(MoveDir1D::Next, false)) ),
        ( NMAP, "X", edit!(EditAction::Delete, MoveType::Column(MoveDir1D::Previous, false)) ),
        ( NMAP, "y", edit_motion!(EditAction::Yank) ),
        ( NMAP, "yy", edit_lines!(EditAction::Yank) ),
        ( NMAP, "Y", edit_lines!(EditAction::Yank) ),
        ( NMAP, "ZZ", window_close_one!(CloseTarget::Single, FocusChange::Current, CloseFlags::WQ) ),
        ( NMAP, "ZQ", window_close_one!(CloseTarget::Single, FocusChange::Current, CloseFlags::FQ) ),
        ( NMAP, "=", edit_motion!(EditAction::Indent(IndentChange::Auto)) ),
        ( NMAP, "==", edit_lines!(EditAction::Indent(IndentChange::Auto)) ),
        ( NMAP, "<", edit_motion!(EditAction::Indent(IndentChange::Decrease(Count::Exact(1)))) ),
        ( NMAP, "<<", edit_lines!(EditAction::Indent(IndentChange::Decrease(Count::Exact(1)))) ),
        ( NMAP, ">", edit_motion!(EditAction::Indent(IndentChange::Increase(Count::Exact(1)))) ),
        ( NMAP, ">>", edit_lines!(EditAction::Indent(IndentChange::Increase(Count::Exact(1)))) ),
        ( NMAP, "~", tilde!() ),
        ( NMAP, ".", act!(Action::Repeat(RepeatType::EditSequence)) ),
        ( NMAP, ":", cmdbar_focus!(CommandType::Command) ),
        ( NMAP, "@{register}", act!(MacroAction::Execute(Count::Contextual).into()) ),
        ( NMAP, "@:", command!(CommandAction::Repeat(Count::Contextual)) ),
        ( NMAP, "@@", act!(MacroAction::Repeat(Count::Contextual).into()) ),
        ( NMAP, "<C-A>", edit!(EditAction::ChangeNumber(NumberChange::IncreaseOne), MoveType::LinePos(MovePosition::End)) ),
        ( NMAP, "<C-C>", normal!() ),
        ( NMAP, "<C-I>", jump!(PositionList::JumpList, MoveDir1D::Next) ),
        ( NMAP, "<C-G>", unmapped!() ),
        ( NMAP, "<C-L>", act!(Action::RedrawScreen) ),
        ( NMAP, "<C-O>", jump!(PositionList::JumpList, MoveDir1D::Previous) ),
        ( NMAP, "<C-R>", history!(HistoryAction::Redo(Count::Contextual)) ),
        ( NMAP, "<C-T>", unmapped!() ),
        ( NMAP, "<C-X>", edit!(EditAction::ChangeNumber(NumberChange::DecreaseOne), MoveType::LinePos(MovePosition::End)) ),
        ( NMAP, "<C-Z>", act!(Action::Suspend) ),
        ( NMAP, "<C-^>", unmapped!() ),
        ( NMAP, "<Del>", edit_nocount!(EditAction::Delete, MoveType::Column(MoveDir1D::Next, false)) ),
        ( NMAP, "<Esc>", normal!() ),
        ( NMAP, "<Insert>", insert!(InsertStyle::Insert) ),

        // Visual, Select mode keys
        ( VMAP, "<C-A>", edit_selection!(EditAction::ChangeNumber(NumberChange::IncreaseOne)) ),
        ( VMAP, "<C-C>", normal!() ),
        ( VMAP, "<C-L>", act!(Action::RedrawScreen) ),
        ( VMAP, "<C-X>", edit_selection!(EditAction::ChangeNumber(NumberChange::DecreaseOne)) ),
        ( VMAP, "<C-Z>", act!(Action::Suspend) ),
        ( VMAP, "<Del>", edit_selection_nocount!(EditAction::Delete) ),
        ( VMAP, "<Esc>", normal!() ),

        // Visual mode keys
        ( XMAP, "A", insert_visual!(SelectionCursorChange::End) ),
        ( XMAP, "c", insert_visual!(SelectionCursorChange::Beginning, EditTarget::Selection) ),
        ( XMAP, "C", change_selection_nochar!(SelectionCursorChange::Beginning, EditTarget::Motion(MoveType::LinePos(MovePosition::End), Count::Exact(0))) ),
        ( XMAP, "d", edit_selection!(EditAction::Delete) ),
        ( XMAP, "D", delete_selection_nochar!(SelectionCursorChange::Beginning, EditTarget::Motion(MoveType::LinePos(MovePosition::End), Count::Exact(0))) ),
        ( XMAP, "gJ", edit_selection!(EditAction::Join(JoinStyle::NoChange)) ),
        ( XMAP, "gn", selection_resize_search!(SelectionResizeStyle::Extend, MoveDir1D::Next) ),
        ( XMAP, "gN", selection_resize_search!(SelectionResizeStyle::Extend, MoveDir1D::Previous) ),
        ( XMAP, "gq", edit_selection!(EditAction::Format) ),
        ( XMAP, "gr", charreplace!(true, EditTarget::Selection) ),
        ( XMAP, "gu", edit_selection!(EditAction::ChangeCase(Case::Lower)) ),
        ( XMAP, "gU", edit_selection!(EditAction::ChangeCase(Case::Upper)) ),
        ( XMAP, "gw", edit_selection!(EditAction::Format) ),
        ( XMAP, "g~", edit_selection!(EditAction::ChangeCase(Case::Toggle)) ),
        ( XMAP, "g<C-A>", edit_selection!(EditAction::ChangeNumber(NumberChange::IncreaseAll)) ),
        ( XMAP, "g<C-X>", edit_selection!(EditAction::ChangeNumber(NumberChange::DecreaseAll)) ),
        ( XMAP, "I", insert_visual!(SelectionCursorChange::Beginning) ),
        ( XMAP, "J", edit_selection!(EditAction::Join(JoinStyle::OneSpace)) ),
        ( XMAP, "K", act!(Action::KeywordLookup) ),
        ( XMAP, "o", selection!(SelectionAction::CursorSet(SelectionCursorChange::SwapAnchor(false))) ),
        ( XMAP, "O", selection!(SelectionAction::CursorSet(SelectionCursorChange::SwapAnchor(true))) ),
        ( XMAP, "p", paste!(MoveDir1D::Next, Count::Contextual, VimMode::Normal) ),
        ( XMAP, "P", paste!(MoveDir1D::Previous, Count::Contextual, VimMode::Normal) ),
        ( XMAP, "r", charreplace!(false, EditTarget::Selection) ),
        ( XMAP, "R", change_selection_lines!() ),
        ( XMAP, "S", change_selection_lines!() ),
        ( XMAP, "u", edit_selection!(EditAction::ChangeCase(Case::Lower)) ),
        ( XMAP, "U", edit_selection!(EditAction::ChangeCase(Case::Upper)) ),
        ( XMAP, "v", visual!(TargetShape::CharWise) ),
        ( XMAP, "V", visual!(TargetShape::LineWise) ),
        ( XMAP, "x", edit_selection!(EditAction::Delete) ),
        ( XMAP, "X", delete_selection_nochar!(SelectionCursorChange::Beginning, EditTarget::Selection) ),
        ( XMAP, "y", edit_selection!(EditAction::Yank) ),
        ( XMAP, "Y", edit_selection_nochar!(EditAction::Yank) ),
        ( XMAP, "~", edit_selection!(EditAction::ChangeCase(Case::Toggle)) ),
        ( XMAP, "=", edit_selection!(EditAction::Indent(IndentChange::Auto)) ),
        ( XMAP, "<", edit_selection!(EditAction::Indent(IndentChange::Decrease(Count::Contextual))) ),
        ( XMAP, ">", edit_selection!(EditAction::Indent(IndentChange::Increase(Count::Contextual))) ),
        ( XMAP, "<C-G>", goto!(VimMode::Select) ),

        // Select mode
        ( SMAP, "<C-G>", goto!(VimMode::Visual) ),
        ( SMAP, "<C-O>", fallthrough!(VimMode::Visual) ),

        // Insert, Command mode
        ( ICMAP, "<C-H>", edit!(EditAction::Delete, MoveType::Column(MoveDir1D::Previous, true)) ),
        ( ICMAP, "<C-K>", iact!(InternalAction::SetCursorChar('?')) ),
        ( ICMAP, "<C-K>{digraph1}", iact!(InternalAction::SetCursorDigraph) ),
        ( ICMAP, "<C-K>{digraph1}{digraph2}", chartype!() ),
        ( ICMAP, "<C-R>", iact!(InternalAction::SetCursorChar('"')) ),
        ( ICMAP, "<C-R>{register}", paste!(MoveDir1D::Previous, 1) ),
        ( ICMAP, "<C-R><C-C>", normal!() ),
        ( ICMAP, "<C-R><C-O>{register}", unmapped!() ),
        ( ICMAP, "<C-R><C-R>{register}", unmapped!() ),
        ( ICMAP, "<C-U>", edit!(EditAction::Delete,  MoveType::LinePos(MovePosition::Beginning), 0) ),
        ( ICMAP, "<C-V>", iact!(InternalAction::SetCursorChar('^')) ),
        ( ICMAP, "<C-V>o{oct<=3}", chartype!() ),
        ( ICMAP, "<C-V>O{oct<=3}", chartype!() ),
        ( ICMAP, "<C-V>x{hex<=2}", chartype!() ),
        ( ICMAP, "<C-V>X{hex<=2}", chartype!() ),
        ( ICMAP, "<C-V>u{hex<=4}", chartype!() ),
        ( ICMAP, "<C-V>U{hex<=8}", chartype!() ),
        ( ICMAP, "<C-V>{dec<=3}", chartype!() ),
        ( ICMAP, "<C-V>{any}", chartype!() ),
        ( ICMAP, "<C-W>", edit!(EditAction::Delete, MoveType::WordBegin(WordStyle::Little, MoveDir1D::Previous)) ),
        ( ICMAP, "<Left>", edit!(EditAction::Motion, MoveType::Column(MoveDir1D::Previous, false)) ),
        ( ICMAP, "<Right>", edit!(EditAction::Motion, MoveType::Column(MoveDir1D::Next, false)) ),
        ( ICMAP, "<Home>", edit!(EditAction::Motion, MoveType::LinePos(MovePosition::Beginning), 0) ),
        ( ICMAP, "<End>", edit!(EditAction::Motion, MoveType::LinePos(MovePosition::End), 0) ),
        ( ICMAP, "<BS>", edit!(EditAction::Delete, MoveType::Column(MoveDir1D::Previous, true)) ),
        ( ICMAP, "<Del>", edit!(EditAction::Delete, MoveType::Column(MoveDir1D::Next, true)) ),

        // Insert Mode
        ( IMAP, "<C-@>", paste_register!(MoveDir1D::Previous, Register::LastInserted, VimMode::Normal) ),
        ( IMAP, "<C-A>", paste_register!(MoveDir1D::Previous, Register::LastInserted) ),
        ( IMAP, "<C-C>", normal!() ),
        ( IMAP, "<C-D>", edit_lines!(EditAction::Indent(IndentChange::Decrease(Count::Exact(1)))) ),
        ( IMAP, "<C-E>", chartype!(Char::CopyLine(MoveDir1D::Next)) ),
        ( IMAP, "<C-G>j", unmapped!() ),
        ( IMAP, "<C-G>k", unmapped!() ),
        ( IMAP, "<C-G>u", unmapped!() ),
        ( IMAP, "<C-G>U", unmapped!() ),
        ( IMAP, "<C-G><C-J>", unmapped!() ),
        ( IMAP, "<C-G><C-K>", unmapped!() ),
        ( IMAP, "<C-G><Down>", unmapped!() ),
        ( IMAP, "<C-G><Up>", unmapped!() ),
        ( IMAP, "<C-N>", act!(Action::Complete(MoveDir1D::Next, true)) ),
        ( IMAP, "<C-O>", fallthrough!(VimMode::Normal) ),
        ( IMAP, "<C-P>", act!(Action::Complete(MoveDir1D::Previous, true)) ),
        ( IMAP, "<C-R><C-P>{register}", unmapped!() ),
        ( IMAP, "<C-T>", edit_lines!(EditAction::Indent(IndentChange::Increase(Count::Exact(1)))) ),
        ( IMAP, "<C-X><C-E>", scroll2d!(MoveDir2D::Down, ScrollSize::Cell) ),
        ( IMAP, "<C-X><C-Y>", scroll2d!(MoveDir2D::Up, ScrollSize::Cell) ),
        ( IMAP, "<C-Y>", chartype!(Char::CopyLine(MoveDir1D::Previous)) ),
        ( IMAP, "<Up>", edit!(EditAction::Motion, MoveType::Line(MoveDir1D::Previous)) ),
        ( IMAP, "<Down>", edit!(EditAction::Motion, MoveType::Line(MoveDir1D::Next)) ),
        ( IMAP, "<S-Up>", scroll2d!(MoveDir2D::Up, ScrollSize::Page) ),
        ( IMAP, "<S-Down>", scroll2d!(MoveDir2D::Down, ScrollSize::Page) ),
        ( IMAP, "<S-Left>", edit!(EditAction::Motion, MoveType::WordBegin(WordStyle::Little, MoveDir1D::Previous)) ),
        ( IMAP, "<S-Right>", edit!(EditAction::Motion, MoveType::WordBegin(WordStyle::Little, MoveDir1D::Next)) ),
        ( IMAP, "<C-Left>", edit_end!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Previous)) ),
        ( IMAP, "<C-Right>", edit_end!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Next)) ),
        ( IMAP, "<Esc>", normal!() ),
        ( IMAP, "<Tab>", chartype!(Char::Single('\t')) ),
        ( IMAP, "<C-Home>", edit!(EditAction::Motion, MoveType::BufferPos(MovePosition::Beginning)) ),
        ( IMAP, "<Insert>", insert!(InsertStyle::Replace) ),
        ( IMAP, "<PageDown>", scroll2d!(MoveDir2D::Down, ScrollSize::Page) ),
        ( IMAP, "<PageUp>", scroll2d!(MoveDir2D::Up, ScrollSize::Page) ),
        ( IMAP, "<C-PageDown>", tab_focus!(FocusChange::Direction1D(MoveDir1D::Next, Count::Exact(1), true), FocusChange::Offset(Count::Contextual, false)) ),
        ( IMAP, "<C-PageUp>", tab_focus!(FocusChange::Direction1D(MoveDir1D::Previous, Count::Contextual, true)) ),

        // Command mode
        ( CMAP, "<C-A>", unmapped!() ),
        ( CMAP, "<C-B>", edit!(EditAction::Motion, MoveType::LinePos(MovePosition::Beginning), 0) ),
        ( CMAP, "<C-C>", command_unfocus!() ),
        ( CMAP, "<C-D>", unmapped!() ),
        ( CMAP, "<C-E>", edit!(EditAction::Motion, MoveType::LinePos(MovePosition::End), 0) ),
        ( CMAP, "<C-G>", unmapped!() ),
        ( CMAP, "<C-L>", unmapped!() ),
        ( CMAP, "<C-N>", unmapped!() ),
        ( CMAP, "<C-P>", unmapped!() ),
        ( CMAP, "<C-\\><C-N>", command_unfocus!() ),
        ( CMAP, "<Esc>", command_unfocus!() ),
        ( CMAP, "<Tab>", act!(Action::Complete(MoveDir1D::Next, false)) ),
        ( CMAP, "<S-Tab>", act!(Action::Complete(MoveDir1D::Previous, false)) ),
        ( CMAP, "<S-Left>", edit!(EditAction::Motion, MoveType::WordBegin(WordStyle::Big, MoveDir1D::Previous)) ),
        ( CMAP, "<C-Left>", edit!(EditAction::Motion, MoveType::WordBegin(WordStyle::Big, MoveDir1D::Previous)) ),
        ( CMAP, "<S-Right>", edit!(EditAction::Motion, MoveType::WordBegin(WordStyle::Big, MoveDir1D::Next)) ),
        ( CMAP, "<C-Right>", edit!(EditAction::Motion, MoveType::WordBegin(WordStyle::Big, MoveDir1D::Next)) ),
        ( CMAP, "<Up>", cmdbar!(CommandBarAction::Recall(MoveDir1D::Previous, Count::Contextual)) ),
        ( CMAP, "<Down>", cmdbar!(CommandBarAction::Recall(MoveDir1D::Next, Count::Contextual)) ),
        ( CMAP, "<S-Up>", cmdbar!(CommandBarAction::Recall(MoveDir1D::Previous, Count::Contextual)) ),
        ( CMAP, "<S-Down>", cmdbar!(CommandBarAction::Recall(MoveDir1D::Next, Count::Contextual)) ),
        ( CMAP, "<PageUp>", cmdbar!(CommandBarAction::Recall(MoveDir1D::Previous, Count::Contextual)) ),
        ( CMAP, "<PageDown>", cmdbar!(CommandBarAction::Recall(MoveDir1D::Next, Count::Contextual)) ),
        ( CMAP, "<Insert>", iact!(InternalAction::SetInsertStyle(InsertStyle::Replace)) ),

        // Operator-Pending mode
        ( OMAP, "gn", unmapped!() ),
        ( OMAP, "gN", unmapped!() ),

        // Internal mode to simplify keypresses allowed after f/F/t/T.
        ( SUFFIX_CHARSRCH, "<C-K>{digraph1}{digraph2}", charsearch_suffix!() ),
        ( SUFFIX_CHARSRCH, "<C-V>o{oct<=3}", charsearch_suffix!() ),
        ( SUFFIX_CHARSRCH, "<C-V>O{oct<=3}", charsearch_suffix!() ),
        ( SUFFIX_CHARSRCH, "<C-V>x{hex<=2}", charsearch_suffix!() ),
        ( SUFFIX_CHARSRCH, "<C-V>X{hex<=2}", charsearch_suffix!() ),
        ( SUFFIX_CHARSRCH, "<C-V>u{hex<=4}", charsearch_suffix!() ),
        ( SUFFIX_CHARSRCH, "<C-V>U{hex<=8}", charsearch_suffix!() ),
        ( SUFFIX_CHARSRCH, "<C-V>{dec<=3}", charsearch_suffix!() ),
        ( SUFFIX_CHARSRCH, "<C-V>{any}", charsearch_suffix!() ),
        ( SUFFIX_CHARSRCH, "{any}", charsearch_suffix!() ),
        ( SUFFIX_CHARSRCH, "<Esc>", act!(Action::NoOp) ),

        // Internal mode to simplify keypresses allowed after r/gr.
        ( SUFFIX_CHARREPL, "<C-K>{digraph1}{digraph2}", charreplace_suffix!() ),
        ( SUFFIX_CHARREPL, "<C-V>o{oct<=3}", charreplace_suffix!() ),
        ( SUFFIX_CHARREPL, "<C-V>O{oct<=3}", charreplace_suffix!() ),
        ( SUFFIX_CHARREPL, "<C-V>x{hex<=2}", charreplace_suffix!() ),
        ( SUFFIX_CHARREPL, "<C-V>X{hex<=2}", charreplace_suffix!() ),
        ( SUFFIX_CHARREPL, "<C-V>u{hex<=4}", charreplace_suffix!() ),
        ( SUFFIX_CHARREPL, "<C-V>U{hex<=8}", charreplace_suffix!() ),
        ( SUFFIX_CHARREPL, "<C-V>{dec<=3}", charreplace_suffix!() ),
        ( SUFFIX_CHARREPL, "{any}", charreplace_suffix!() ),
        ( SUFFIX_CHARREPL, "<C-C>", normal!() ),
        ( SUFFIX_CHARREPL, "<Esc>", act!(Action::NoOp) ),
        ( SUFFIX_CHARREPL, "<C-E>", charreplace_suffix!(Char::CopyLine(MoveDir1D::Previous)) ),
        ( SUFFIX_CHARREPL, "<C-Y>", charreplace_suffix!(Char::CopyLine(MoveDir1D::Next)) ),
    ].to_vec()
}

#[rustfmt::skip]
fn default_pfxs<P: Application>() -> Vec<(MappedModes, &'static str, Option<InputStep<P>>)> {
    [
        // Normal, Visual and Operator-Pending mode commands can be prefixed w/ a count.
        ( NXOMAP, "{count}", Some(iact!(InternalAction::SaveCounting)) ),

        // Normal and Visual mode keys can be prefixed w/ a register.
        ( NXMAP, "\"{register}", None ),

        // Operator-Pending mode keys can be prefixed w/ the forced-motion keys.
        ( OMAP, "v", Some(shape!(TargetShape::CharWise)) ),
        ( OMAP, "V", Some(shape!(TargetShape::LineWise)) ),
        ( OMAP, "<C-V>", Some(shape!(TargetShape::BlockWise)) ),
    ].to_vec()
}

#[rustfmt::skip]
fn default_enter<P: Application>() -> Vec<(MappedModes, &'static str, InputStep<P>)> {
    [
        // <Enter> in Normal, Visual, and Operator-Pending mode moves to next line.
        ( MAP, "<Enter>", edit_end!(MoveType::FirstWord(MoveDir1D::Next)) ),

        // <Enter> in Insert mode types a newlines.
        ( IMAP, "<Enter>", chartype!(Char::Single('\n')) ),

        // <Enter> in Command mode submits the command.
        ( CMAP, "<Enter>", cmdbar!(CommandBarAction::Submit, VimMode::Normal) ),
    ].to_vec()
}

#[rustfmt::skip]
fn default_search<P: Application>() -> Vec<(MappedModes, &'static str, InputStep<P>)> {
    [
        ( NXOMAP, "n", edit_search_end!(SearchType::Regex, MoveDirMod::Same) ),
        ( NXOMAP, "N", edit_search_end!(SearchType::Regex, MoveDirMod::Flip) ),
    ].to_vec()
}

#[rustfmt::skip]
fn submit_on_enter<P: Application>() -> Vec<(MappedModes, &'static str, InputStep<P>)> {
    [
        // <Enter> in Normal and Visual mode submits contents.
        ( NVMAP, "<Enter>", cmdbar!(CommandBarAction::Submit, VimMode::Normal) ),

        // <Enter> in Insert mode submits contents and stays in Insert mode.
        ( IMAP, "<Enter>", cmdbar!(CommandBarAction::Submit, VimMode::Insert) ),

        // <Enter> in Command mode submits the command.
        ( CMAP, "<Enter>", cmdbar!(CommandBarAction::Submit, VimMode::Normal) ),

        // <Enter> in Operator-Pending mode moves to the next line.
        ( OMAP, "<Enter>", edit_end!(MoveType::FirstWord(MoveDir1D::Next)) ),
    ].to_vec()
}

#[rustfmt::skip]
fn search_is_action<P: Application>() -> Vec<(MappedModes, &'static str, InputStep<P>)> {
    [
        // Perform an application-level search in Normal mode.
        ( NMAP, "n", act!(Action::Search(MoveDirMod::Same, Count::Contextual)) ),
        ( NMAP, "N", act!(Action::Search(MoveDirMod::Flip, Count::Contextual)) ),

        // Perform text search in Operator-Pending mode.
        ( OMAP, "n", edit_search_end!(SearchType::Regex, MoveDirMod::Same) ),
        ( OMAP, "N", edit_search_end!(SearchType::Regex, MoveDirMod::Flip) ),

        // Perform text search in Visual mode.
        ( XMAP, "n", edit_search_end!(SearchType::Regex, MoveDirMod::Same) ),
        ( XMAP, "N", edit_search_end!(SearchType::Regex, MoveDirMod::Flip) ),
    ].to_vec()
}

#[inline]
fn add_prefix<P: Application>(
    machine: &mut VimMachine<TerminalKey, P>,
    modes: &MappedModes,
    keys: &str,
    action: &Option<InputStep<P>>,
) {
    let (_, evs) = parse(keys).expect(&format!("invalid vim keybinding: {}", keys));
    let modes = modes.split();

    for mode in modes {
        machine.add_prefix(mode, &evs, &action);
    }
}

#[inline]
fn add_mapping<P: Application>(
    machine: &mut VimMachine<TerminalKey, P>,
    modes: &MappedModes,
    keys: &str,
    action: &InputStep<P>,
) {
    let (_, evs) = parse(keys).expect(&format!("invalid vim keybinding: {}", keys));
    let modes = modes.split();

    for mode in modes {
        machine.add_mapping(mode, &evs, &action);
    }
}

/// A configurable collection of Vim bindings that can be added to a [ModalMachine].
#[derive(Debug)]
pub struct VimBindings<P: Application> {
    prefixes: Vec<(MappedModes, &'static str, Option<InputStep<P>>)>,
    mappings: Vec<(MappedModes, &'static str, InputStep<P>)>,
    enter: Vec<(MappedModes, &'static str, InputStep<P>)>,
    search: Vec<(MappedModes, &'static str, InputStep<P>)>,
}

impl<P: Application> VimBindings<P> {
    /// Remap the Enter key in Normal, Visual, Select, and Insert mode to
    /// [submit](CommandBarAction::Submit) instead.
    pub fn submit_on_enter(mut self) -> Self {
        self.enter = submit_on_enter();
        self
    }

    /// Remap `n` and `N` in Normal mode to perform [Action::Search] instead.
    pub fn search_is_action(mut self) -> Self {
        self.search = search_is_action();
        self
    }
}

impl<P: Application> Default for VimBindings<P> {
    fn default() -> Self {
        VimBindings {
            prefixes: default_pfxs(),
            mappings: default_keys(),
            enter: default_enter(),
            search: default_search(),
        }
    }
}

impl<P: Application> InputBindings<TerminalKey, InputStep<P>> for VimBindings<P> {
    fn setup(&self, machine: &mut VimMachine<TerminalKey, P>) {
        for (modes, keys, action) in self.prefixes.iter() {
            add_prefix(machine, modes, keys, action);
        }

        for (modes, keys, action) in self.mappings.iter() {
            add_mapping(machine, modes, keys, action);
        }

        for (modes, keys, action) in self.enter.iter() {
            add_mapping(machine, modes, keys, action);
        }

        for (modes, keys, action) in self.search.iter() {
            add_mapping(machine, modes, keys, action);
        }
    }
}

/// Manage Vim keybindings and modes.
pub type VimMachine<Key, T = ()> = ModalMachine<Key, InputStep<T>>;

impl<P: Application> Default for VimMachine<TerminalKey, P> {
    fn default() -> Self {
        ModalMachine::from_bindings::<VimBindings<P>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::base::{CursorCloseTarget, Mark, Register};
    use crate::input::bindings::BindingMachine;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    macro_rules! assert_normal {
        ($mm: expr, $ctx: expr) => {
            let mut keep = crate::input::InputContext::take(&mut $ctx);
            $ctx.persist.shape = None;
            $ctx.persist.insert = None;
            assert_pop2!($mm, CHECKPOINT, $ctx);
            assert_eq!($mm.mode(), VimMode::Normal);
            std::mem::swap(&mut keep, &mut $ctx);
        };
    }

    macro_rules! mvop {
        ($ea: expr, $mt: expr) => {
            Action::Edit(
                Specifier::Exact($ea.clone()),
                EditTarget::Motion($mt.clone(), Count::Contextual),
            )
        };
        ($ea: expr, $mt: expr, $c: literal) => {
            Action::Edit(
                Specifier::Exact($ea.clone()),
                EditTarget::Motion($mt.clone(), Count::Exact($c)),
            )
        };
        ($ea: expr, $mt: expr, $c: expr) => {
            Action::Edit(Specifier::Exact($ea.clone()), EditTarget::Motion($mt.clone(), $c))
        };
    }

    macro_rules! mv {
        ($mt: expr) => {
            Action::Edit(Specifier::Contextual, EditTarget::Motion($mt.clone(), Count::Contextual))
        };
        ($mt: expr, $c: literal) => {
            Action::Edit(Specifier::Contextual, EditTarget::Motion($mt.clone(), Count::Exact($c)))
        };
        ($mt: expr, $c: expr) => {
            Action::Edit(Specifier::Contextual, EditTarget::Motion($mt.clone(), $c))
        };
    }

    macro_rules! rangeop {
        ($ea: expr, $rt: expr) => {
            Action::Edit(Specifier::Exact($ea), EditTarget::Range($rt, true, Count::Contextual))
        };
        ($ea: expr, $rt: expr, $inc: expr) => {
            Action::Edit(Specifier::Exact($ea), EditTarget::Range($rt, $inc, Count::Contextual))
        };
        ($ea: expr, $rt: expr, $inc: expr, $c: literal) => {
            Action::Edit(Specifier::Exact($ea), EditTarget::Range($rt, $inc, Count::Exact($c)))
        };
        ($ea: expr, $rt: expr, $inc: expr, $c: expr) => {
            Action::Edit(Specifier::Exact($ea), EditTarget::Range($rt, $inc, $c))
        };
    }

    macro_rules! range {
        ($rt: expr) => {
            Action::Edit(Specifier::Contextual, EditTarget::Range($rt, true, Count::Contextual))
        };
        ($rt: expr, $inc: expr) => {
            Action::Edit(Specifier::Contextual, EditTarget::Range($rt, $inc, Count::Contextual))
        };
        ($rt: expr, $inc: expr, $c: literal) => {
            Action::Edit(Specifier::Contextual, EditTarget::Range($rt, $inc, Count::Exact($c)))
        };
        ($rt: expr, $inc: expr, $c: expr) => {
            Action::Edit(Specifier::Contextual, EditTarget::Range($rt, $inc, $c))
        };
    }

    macro_rules! typechar {
        ($c: literal) => {
            Action::InsertText(InsertTextAction::Type(
                Char::Single($c).into(),
                MoveDir1D::Previous,
                1.into(),
            ))
        };
    }

    const CURRENT_POS: Action =
        Action::Edit(Specifier::Exact(EditAction::Motion), EditTarget::CurrentPosition);
    const COLUMN_PREV: Action = Action::Edit(
        Specifier::Exact(EditAction::Motion),
        EditTarget::Motion(MoveType::Column(MoveDir1D::Previous, false), Count::Exact(1)),
    );
    const CHECKPOINT: Action = Action::History(HistoryAction::Checkpoint);
    const CMDBAR: Action = Action::CommandBar(CommandBarAction::Focus(CommandType::Command));
    const CMDBAR_ABORT: Action = Action::CommandBar(CommandBarAction::Abort);
    const CMDBAR_SEARCH_NEXT: Action =
        Action::CommandBar(CommandBarAction::Focus(CommandType::Search(MoveDir1D::Next, false)));
    const CMDBAR_SEARCH_PREV: Action = Action::CommandBar(CommandBarAction::Focus(
        CommandType::Search(MoveDir1D::Previous, false),
    ));
    const CURSOR_CLOSE: Action = Action::Cursor(CursorAction::Close(CursorCloseTarget::Followers));
    const CURSOR_SPLIT: Action = Action::Cursor(CursorAction::Split(Count::Contextual));
    const SEL_SPLIT: Action = Action::Selection(SelectionAction::Split(
        SelectionSplitStyle::Lines,
        TargetShapeFilter::ALL,
    ));
    const BLOCK_SPLIT: Action = Action::Selection(SelectionAction::Split(
        SelectionSplitStyle::Lines,
        TargetShapeFilter::BLOCK,
    ));
    const BLOCK_BEG: Action =
        Action::Selection(SelectionAction::CursorSet(SelectionCursorChange::Beginning));
    const BLOCK_END: Action =
        Action::Selection(SelectionAction::CursorSet(SelectionCursorChange::End));
    const TYPE_CONTEXTUAL: Action = Action::InsertText(InsertTextAction::Type(
        Specifier::Contextual,
        MoveDir1D::Previous,
        Count::Exact(1),
    ));

    #[test]
    fn test_transitions_normal() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        let op = EditAction::Motion;

        // Starts in Normal mode
        assert_eq!(vm.mode(), VimMode::Normal);

        // Normal -> Insert mode using "i".
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('i'));
        assert_pop2!(vm, CURSOR_SPLIT, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Insert -> Normal mode using Escape.
        ctx.persist.insert = None;
        vm.input_key(key!(KeyCode::Esc));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Normal -> Insert mode using Insert.
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!(KeyCode::Insert));
        assert_pop2!(vm, CURSOR_SPLIT, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Insert -> Normal mode using ^C.
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Normal -> Insert mode using "gI".
        let mov = mvop!(op, MoveType::LinePos(MovePosition::Beginning), 0);
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('g'));
        vm.input_key(key!('I'));
        assert_pop1!(vm, mov, ctx);
        assert_pop2!(vm, CURSOR_SPLIT, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Insert -> Normal mode using ^C.
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Normal -> Insert mode using "A".
        let mov = mvop!(op, MoveType::LinePos(MovePosition::End), 0);
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('A'));
        assert_pop1!(vm, mov, ctx);
        assert_pop2!(vm, CURSOR_SPLIT, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Insert -> Normal mode using ^C.
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Normal -> Insert mode using "I".
        let mov = mvop!(op, MoveType::FirstWord(MoveDir1D::Next), 0);
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('I'));
        assert_pop1!(vm, mov, ctx);
        assert_pop2!(vm, CURSOR_SPLIT, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Insert -> Normal mode using ^C.
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Normal -> Replace mode using "R".
        ctx.persist.insert = Some(InsertStyle::Replace);
        vm.input_key(key!('R'));
        assert_pop2!(vm, CURSOR_SPLIT, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Replace -> Insert mode using Insert.
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!(KeyCode::Insert));
        assert_pop2!(vm, CURSOR_SPLIT, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);
        assert_eq!(vm.context().persist.insert, Some(InsertStyle::Insert));

        // Insert -> Replace mode using Insert.
        ctx.persist.insert = Some(InsertStyle::Replace);
        vm.input_key(key!(KeyCode::Insert));
        assert_pop2!(vm, CURSOR_SPLIT, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);
        assert_eq!(vm.context().persist.insert, Some(InsertStyle::Replace));

        // Replace -> Normal mode using ^C.
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Normal -> Visual mode (charwise) using "v".
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Visual -> Normal mode using ^C.
        ctx.persist.shape = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Normal -> Visual mode (linewise) using "V".
        ctx.persist.shape = Some(TargetShape::LineWise);
        vm.input_key(key!('V'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Visual -> Normal mode using Escape.
        ctx.persist.shape = None;
        vm.input_key(key!(KeyCode::Esc));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Normal -> Visual mode (blockwise) using ^V
        ctx.persist.shape = Some(TargetShape::BlockWise);
        vm.input_key(ctl!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);
    }

    #[test]
    fn test_transitions_command() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // Starts in Normal mode
        assert_eq!(vm.mode(), VimMode::Normal);

        // Move to Command mode using ":".
        vm.input_key(key!(':'));
        assert_pop2!(vm, CMDBAR, ctx);
        assert_eq!(vm.mode(), VimMode::Command);

        // Unmapped key types that character.
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!(':'));
        assert_pop2!(vm, typechar!(':'), ctx);
        assert_eq!(vm.mode(), VimMode::Command);

        // Unmapped key types that character.
        vm.input_key(key!('a'));
        assert_pop2!(vm, typechar!('a'), ctx);
        assert_eq!(vm.mode(), VimMode::Command);

        // Unmapped key types that character.
        vm.input_key(key!('A'));
        assert_pop2!(vm, typechar!('A'), ctx);
        assert_eq!(vm.mode(), VimMode::Command);

        // Go back to Normal mode via Escape.
        vm.input_key(key!(KeyCode::Esc));
        assert_pop1!(vm, CMDBAR_ABORT, ctx);

        ctx.persist.insert = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Move to Command mode (forward search) using "/".
        vm.input_key(key!('/'));
        assert_pop2!(vm, CMDBAR_SEARCH_NEXT, ctx);
        assert_eq!(vm.mode(), VimMode::Command);

        // Unmapped key types that character.
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('1'));
        assert_pop2!(vm, typechar!('1'), ctx);
        assert_eq!(vm.mode(), VimMode::Command);

        // Go back to Normal mode via ^C.
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CMDBAR_ABORT, ctx);

        ctx.persist.insert = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Move to Command mode (reverse search) using "?".
        ctx.persist.regexsearch_dir = MoveDir1D::Previous;
        vm.input_key(key!('?'));
        assert_pop2!(vm, CMDBAR_SEARCH_PREV, ctx);
        assert_eq!(vm.mode(), VimMode::Command);

        // Unmapped key types that character.
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('^'));
        assert_pop2!(vm, typechar!('^'), ctx);
        assert_eq!(vm.mode(), VimMode::Command);

        // Go back to Normal mode via ^C.
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CMDBAR_ABORT, ctx);

        ctx.persist.insert = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_transitions_visual() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // Move to Visual mode (charwise) and back using "v".
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        ctx.persist.shape = None;
        vm.input_key(key!('v'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Move to Visual mode (linewise) and back using "V".
        ctx.persist.shape = Some(TargetShape::LineWise);
        vm.input_key(key!('V'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        ctx.persist.shape = None;
        vm.input_key(key!('V'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Move to Visual mode (blockwise) and back using ^V.
        ctx.persist.shape = Some(TargetShape::BlockWise);
        vm.input_key(ctl!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        ctx.persist.shape = None;
        vm.input_key(ctl!('v'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Cycle through the different Visual modes.
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        ctx.persist.shape = Some(TargetShape::LineWise);
        vm.input_key(key!('V'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        ctx.persist.shape = Some(TargetShape::BlockWise);
        vm.input_key(ctl!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        ctx.persist.shape = Some(TargetShape::LineWise);
        vm.input_key(key!('V'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        ctx.persist.shape = None;
        vm.input_key(key!('v'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_transitions_visual_select() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // Test charwise shapes.
        ctx.persist.shape = Some(TargetShape::CharWise);

        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Move to Select mode (charwise) and back using ^G.
        vm.input_key(ctl!('g'));
        assert_eq!(vm.pop(), None);
        assert_eq!(vm.mode(), VimMode::Select);

        vm.input_key(ctl!('g'));
        assert_eq!(vm.pop(), None);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Test linewise shapes.
        ctx.persist.shape = Some(TargetShape::LineWise);

        // Move to Visual mode (linewise) using "V".
        vm.input_key(key!('V'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Move to Select mode (linewise) and back using ^G.
        vm.input_key(ctl!('g'));
        assert_eq!(vm.pop(), None);
        assert_eq!(vm.mode(), VimMode::Select);

        vm.input_key(ctl!('g'));
        assert_eq!(vm.pop(), None);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Test blockwise shapes.
        ctx.persist.shape = Some(TargetShape::BlockWise);

        // Move to Visual mode (blockwise) using ^V.
        vm.input_key(ctl!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Move to Select mode (blockwise) and back using ^G.
        vm.input_key(ctl!('g'));
        assert_eq!(vm.pop(), None);
        assert_eq!(vm.mode(), VimMode::Select);

        vm.input_key(ctl!('g'));
        assert_eq!(vm.pop(), None);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Back to Select mode using ^G.
        vm.input_key(ctl!('g'));
        assert_eq!(vm.pop(), None);
        assert_eq!(vm.mode(), VimMode::Select);

        // Back to Normal mode by repeating ^V.
        ctx.persist.shape = None;
        vm.input_key(ctl!('v'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_transitions_select() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // Enter Select mode (charwise) via "gh".
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('g'));
        vm.input_key(key!('h'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Select);

        // Select text using the Right arrow key.
        let mov = mv!(MoveType::Column(MoveDir1D::Next, false));
        vm.input_key(key!(KeyCode::Right));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Select);

        // Enter a single Visual mode command by using ^O.
        vm.input_key(ctl!('o'));
        assert_eq!(vm.pop(), None);
        assert_eq!(vm.mode(), VimMode::Select);

        let mov = mv!(MoveType::Column(MoveDir1D::Next, false));
        vm.input_key(key!('l'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Select);

        let mov = mv!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Next));
        vm.input_key(key!(KeyCode::Right, KeyModifiers::SHIFT));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Select);

        ctx.persist.insert = Some(InsertStyle::Insert);

        vm.input_key(key!('H'));
        assert_pop1!(vm, Action::Edit(EditAction::Delete.into(), EditTarget::Selection), ctx);
        assert_pop1!(vm, typechar!('H'), ctx);

        ctx.persist.shape = None;
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        vm.input_key(key!('l'));
        assert_pop2!(vm, typechar!('l'), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Back to Normal mode.
        ctx.persist.insert = None;

        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Move to Select mode (blockwise) using g^H.
        ctx.persist.shape = Some(TargetShape::BlockWise);
        vm.input_key(key!('g'));
        vm.input_key(ctl!('h'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Select);

        // Move the cursor down one line, doing a blockwise selection.
        let mov = mv!(MoveType::Line(MoveDir1D::Next));
        vm.input_key(key!(KeyCode::Down));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Select);

        // Using ^O to repeat g^H goes back to Normal mode.
        ctx.persist.shape = None;

        vm.input_key(ctl!('o'));
        assert_eq!(vm.pop(), None);
        assert_eq!(vm.mode(), VimMode::Select);

        vm.input_key(key!('g'));
        assert_eq!(vm.pop(), None);
        assert_eq!(vm.mode(), VimMode::Select);

        vm.input_key(ctl!('h'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_count() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        let mov = mv!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Next));

        // "0" does not count, but moves to the first column.
        vm.input_key(key!('0'));
        assert_pop1!(vm, mv!(MoveType::LinePos(MovePosition::Beginning), 0), ctx);
        assert_normal!(vm, ctx);

        // Test initial non-"0" number.
        ctx.action.count = Some(5);
        vm.input_key(key!('5'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // Test "0" after initial non-"0" number.
        ctx.action.count = Some(10);
        vm.input_key(key!('1'));
        vm.input_key(key!('0'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // Test multiple "0" keys after initial non-"0" number.
        ctx.action.count = Some(100);
        vm.input_key(key!('1'));
        vm.input_key(key!('0'));
        vm.input_key(key!('0'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // Test every number.
        ctx.action.count = Some(1234567890);
        vm.input_key(key!('1'));
        vm.input_key(key!('2'));
        vm.input_key(key!('3'));
        vm.input_key(key!('4'));
        vm.input_key(key!('5'));
        vm.input_key(key!('6'));
        vm.input_key(key!('7'));
        vm.input_key(key!('8'));
        vm.input_key(key!('9'));
        vm.input_key(key!('0'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.operation = EditAction::Delete;

        // Operator-Pending mode count is multiplied by Normal mode count.
        ctx.action.count = Some(4);
        vm.input_key(key!('2'));
        vm.input_key(key!('d'));
        vm.input_key(key!('2'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.count = Some(16);
        vm.input_key(key!('8'));
        vm.input_key(key!('d'));
        vm.input_key(key!('2'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // Initial "0" in Operator-Pending mode is a movement.
        let mov = mv!(MoveType::LinePos(MovePosition::Beginning), 0);

        ctx.action.count = None;
        vm.input_key(key!('d'));
        vm.input_key(key!('0'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.count = Some(2);
        vm.input_key(key!('2'));
        vm.input_key(key!('d'));
        vm.input_key(key!('0'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_register() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        let op = EditAction::Yank;
        let mov = rangeop!(op, RangeType::Line);
        ctx.action.operation = EditAction::Yank;

        ctx.action.register = None;
        vm.input_key(key!('y'));
        vm.input_key(key!('y'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.register = Some(Register::Named('a'));
        ctx.action.register_append = false;
        vm.input_key(key!('"'));
        vm.input_key(key!('a'));
        vm.input_key(key!('y'));
        vm.input_key(key!('y'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.register = Some(Register::Named('a'));
        ctx.action.register_append = true;
        vm.input_key(key!('"'));
        vm.input_key(key!('A'));
        vm.input_key(key!('y'));
        vm.input_key(key!('y'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.register = Some(Register::LastYanked);
        ctx.action.register_append = false;
        vm.input_key(key!('"'));
        vm.input_key(key!('0'));
        vm.input_key(key!('y'));
        vm.input_key(key!('y'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.register = Some(Register::RecentlyDeleted(4));
        vm.input_key(key!('"'));
        vm.input_key(key!('5'));
        vm.input_key(key!('y'));
        vm.input_key(key!('y'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.register = Some(Register::Unnamed);
        vm.input_key(key!('"'));
        vm.input_key(key!('"'));
        vm.input_key(key!('y'));
        vm.input_key(key!('y'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.register = Some(Register::Blackhole);
        vm.input_key(key!('"'));
        vm.input_key(key!('_'));
        vm.input_key(key!('y'));
        vm.input_key(key!('y'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_mark() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // Create local mark 'c.
        ctx.action.mark = Some(Mark::BufferNamed('c'));
        vm.input_key(key!('m'));
        vm.input_key(key!('c'));
        assert_pop1!(vm, Action::Mark(Specifier::Contextual), ctx);
        assert_normal!(vm, ctx);

        // Create global mark 'C.
        ctx.action.mark = Some(Mark::GlobalNamed('C'));
        vm.input_key(key!('m'));
        vm.input_key(key!('C'));
        assert_pop1!(vm, Action::Mark(Specifier::Contextual), ctx);
        assert_normal!(vm, ctx);

        // Go to the line of last inserted text.
        let target = EditTarget::LineJump(Specifier::Contextual);
        let mov = Action::Edit(Specifier::Contextual, target);
        ctx.action.mark = Some(Mark::LastInserted);
        vm.input_key(key!('\''));
        vm.input_key(key!('^'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // Go to the column of the end of the last visual selection.
        let target = EditTarget::CharJump(Specifier::Contextual);
        let mov = Action::Edit(Specifier::Contextual, target);
        ctx.action.mark = Some(Mark::VisualEnd);
        vm.input_key(key!('`'));
        vm.input_key(key!('>'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_normal_ops() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        let mov = mv!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Next));

        ctx.action.operation = EditAction::Yank;
        vm.input_key(key!('y'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.operation = EditAction::Format;
        vm.input_key(key!('g'));
        vm.input_key(key!('q'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.operation = EditAction::ChangeCase(Case::Lower);
        vm.input_key(key!('g'));
        vm.input_key(key!('u'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.operation = EditAction::ChangeCase(Case::Upper);
        vm.input_key(key!('g'));
        vm.input_key(key!('U'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.operation = EditAction::ChangeCase(Case::Toggle);
        vm.input_key(key!('g'));
        vm.input_key(key!('~'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.operation = EditAction::Indent(IndentChange::Auto);
        vm.input_key(key!('='));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.operation = EditAction::Indent(IndentChange::Decrease(Count::Exact(1)));
        vm.input_key(key!('<'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.operation = EditAction::Indent(IndentChange::Increase(Count::Exact(1)));
        vm.input_key(key!('>'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        let mov = range!(RangeType::Word(WordStyle::Little));

        ctx.action.operation = EditAction::Format;
        vm.input_key(key!('g'));
        vm.input_key(key!('w'));
        vm.input_key(key!('a'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.operation = EditAction::Motion;

        let op = EditAction::Join(JoinStyle::OneSpace);
        let lines = rangeop!(op, RangeType::Line);
        vm.input_key(key!('J'));
        assert_pop1!(vm, lines, ctx);
        assert_normal!(vm, ctx);

        let op = EditAction::Join(JoinStyle::NoChange);
        let lines = rangeop!(op, RangeType::Line);
        vm.input_key(key!('g'));
        vm.input_key(key!('J'));
        assert_pop1!(vm, lines, ctx);
        assert_normal!(vm, ctx);

        let col = MoveType::Column(MoveDir1D::Next, false);
        ctx.action.cursor_end = Some(CursorEnd::End);
        vm.input_key(key!('~'));
        assert_pop1!(vm, mvop!(EditAction::ChangeCase(Case::Toggle), col), ctx);
        assert_normal!(vm, ctx);

        let mov = mv!(MoveType::Column(MoveDir1D::Next, false));
        ctx.action.cursor_end = None;
        ctx.action.operation = EditAction::Replace(false);
        ctx.action.replace = Some('A'.into());
        ctx.ch.any = Some(key!('A').into());
        vm.input_key(key!('r'));
        vm.input_key(key!('A'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);
        ctx.action.operation = EditAction::Motion;
        ctx.action.replace = None;
        ctx.ch.any = None;

        let op = EditAction::ChangeNumber(NumberChange::IncreaseOne);
        let mov = mvop!(op, MoveType::LinePos(MovePosition::End));
        vm.input_key(ctl!('a'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        let op = EditAction::ChangeNumber(NumberChange::DecreaseOne);
        let mov = mvop!(op, MoveType::LinePos(MovePosition::End));
        vm.input_key(ctl!('x'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_delete_ops() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        let op = EditAction::Delete;

        let mov = mv!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Next));
        ctx.action.operation = op.clone();
        vm.input_key(key!('d'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        ctx.action.operation = EditAction::Motion;

        let movend = mvop!(op, MoveType::LinePos(MovePosition::End));
        vm.input_key(key!('D'));
        assert_pop1!(vm, movend, ctx);
        assert_normal!(vm, ctx);

        let mov = mvop!(op, MoveType::Column(MoveDir1D::Next, false));
        vm.input_key(key!(KeyCode::Delete));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        let mov = mvop!(op, MoveType::Column(MoveDir1D::Next, false));
        vm.input_key(key!('x'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        let mov = mvop!(op, MoveType::Column(MoveDir1D::Previous, false));
        vm.input_key(key!('X'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_change_ops() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // Change a word around the cursor with "caw".
        let mov = range!(RangeType::Word(WordStyle::Little));
        ctx.action.operation = EditAction::Delete;
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('c'));
        vm.input_key(key!('a'));
        vm.input_key(key!('w'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Move back to Normal mode via ^C.
        ctx.action.operation = EditAction::Motion;
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Change from cursor to end of a word with "cw".
        let mov = mv!(MoveType::WordEnd(WordStyle::Little, MoveDir1D::Next));
        ctx.action.operation = EditAction::Delete;
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('c'));
        vm.input_key(key!('w'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Move back to Normal mode via ^C.
        ctx.action.operation = EditAction::Motion;
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Change from cursor to end of a WORD with "cW".
        let mov = mv!(MoveType::WordEnd(WordStyle::Big, MoveDir1D::Next));
        ctx.action.operation = EditAction::Delete;
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('c'));
        vm.input_key(key!('W'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Move back to Normal mode via ^C.
        ctx.action.operation = EditAction::Motion;
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Substitute a character with "s".
        let op = EditAction::Delete;
        let mov = mvop!(op, MoveType::Column(MoveDir1D::Next, false));
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('s'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Move back to Normal mode via ^C.
        ctx.action.operation = EditAction::Motion;
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Change from cursor to end of the line with "C".
        let op = EditAction::Delete;
        let mov = mvop!(op, MoveType::LinePos(MovePosition::End));
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('C'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Move back to Normal mode via ^C.
        ctx.action.operation = EditAction::Motion;
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Change the current line with "S".
        let op = EditAction::Delete;
        let mov = rangeop!(op, RangeType::Line);
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('S'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Move back to Normal mode via ^C.
        ctx.action.operation = EditAction::Motion;
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Pressing c^C should not go to Insert mode.
        vm.input_key(key!('c'));
        vm.input_key(ctl!('c'));
        assert_normal!(vm, ctx);

        // We should have reset, and can now type a Normal mode command.
        vm.input_key(ctl!('l'));
        assert_pop1!(vm, Action::RedrawScreen, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_normal_motion_charsearch() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        let same_target =
            EditTarget::Search(SearchType::Char(false), MoveDirMod::Same, Count::Contextual);
        let flip_target =
            EditTarget::Search(SearchType::Char(false), MoveDirMod::Flip, Count::Contextual);

        let same = Action::Edit(Specifier::Contextual, same_target.clone());
        let flip = Action::Edit(Specifier::Contextual, flip_target.clone());

        // "fa" should update search params, and then continue character search.
        ctx.persist.charsearch_params = (MoveDir1D::Next, true);
        ctx.persist.charsearch = Some('a'.into());
        ctx.ch.any = Some(key!('a').into());
        vm.input_key(key!('f'));
        vm.input_key(key!('a'));
        assert_pop1!(vm, same, ctx);
        assert_normal!(vm, ctx);

        // ";" should continue character search.
        ctx.persist.charsearch_params = (MoveDir1D::Next, true);
        ctx.persist.charsearch = Some('a'.into());
        ctx.ch.any = None;
        vm.input_key(key!(';'));
        assert_pop1!(vm, same, ctx);
        assert_normal!(vm, ctx);

        // "," should continue character search in reverse direction.
        ctx.persist.charsearch_params = (MoveDir1D::Next, true);
        ctx.persist.charsearch = Some('a'.into());
        ctx.ch.any = None;
        vm.input_key(key!(','));
        assert_pop1!(vm, flip, ctx);
        assert_normal!(vm, ctx);

        // "T<C-V>o125" should update params and continue search for codepoint.
        ctx.persist.charsearch_params = (MoveDir1D::Previous, false);
        ctx.persist.charsearch = Some('U'.into());
        ctx.ch.oct = Some(85);
        vm.input_key(key!('T'));
        vm.input_key(ctl!('v'));
        vm.input_key(key!('o'));
        vm.input_key(key!('1'));
        vm.input_key(key!('2'));
        vm.input_key(key!('5'));
        assert_pop1!(vm, same, ctx);
        assert_normal!(vm, ctx);

        // ";" should continue search.
        ctx.persist.charsearch_params = (MoveDir1D::Previous, false);
        ctx.persist.charsearch = Some('U'.into());
        ctx.ch.oct = None;
        vm.input_key(key!(';'));
        assert_pop1!(vm, same, ctx);
        assert_normal!(vm, ctx);

        // "F<C-K>Z<" should update params and continue search for digraph.
        ctx.persist.charsearch_params = (MoveDir1D::Previous, true);
        ctx.persist.charsearch = Some(Char::Digraph('Z', '<'));
        ctx.ch.digraph1 = Some('Z');
        ctx.ch.digraph2 = Some('<');
        vm.input_key(key!('F'));
        vm.input_key(ctl!('k'));
        vm.input_key(key!('Z'));
        vm.input_key(key!('<'));
        assert_pop1!(vm, same, ctx);
        assert_normal!(vm, ctx);

        // "," should continue search in reverse direction.
        ctx.persist.charsearch_params = (MoveDir1D::Previous, true);
        ctx.persist.charsearch = Some(Char::Digraph('Z', '<'));
        ctx.ch.digraph1 = None;
        ctx.ch.digraph2 = None;
        vm.input_key(key!(','));
        assert_pop1!(vm, flip, ctx);
        assert_normal!(vm, ctx);

        // "t<Esc>" should do nothing leave persistent search parameters alone.
        vm.input_key(key!('t'));
        vm.input_key(key!(KeyCode::Esc));
        ctx.persist.charsearch_params = (MoveDir1D::Previous, true);
        ctx.persist.charsearch = Some(Char::Digraph('Z', '<'));
        ctx.action.charsearch_params = Some((MoveDir1D::Next, false));
        ctx.action.target = Some(same_target);
        assert_pop1!(vm, Action::NoOp, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_normal_motion_special_key() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // <C-H>
        let mov = mv!(MoveType::Column(MoveDir1D::Previous, true));
        vm.input_key(ctl!('h'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <C-?> (backspace)
        let mov = mv!(MoveType::Column(MoveDir1D::Previous, true));
        vm.input_key(key!(KeyCode::Backspace));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // Space
        let mov = mv!(MoveType::Column(MoveDir1D::Next, true));
        vm.input_key(key!(' '));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <C-J> (newline)
        let mov = mv!(MoveType::Line(MoveDir1D::Next));
        vm.input_key(key!('\n'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <C-N>
        let mov = mv!(MoveType::Line(MoveDir1D::Next));
        vm.input_key(ctl!('n'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <C-P>
        let mov = mv!(MoveType::Line(MoveDir1D::Previous));
        vm.input_key(ctl!('p'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <Up>
        let mov = mv!(MoveType::Line(MoveDir1D::Previous));
        vm.input_key(key!(KeyCode::Up));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <Down>
        let mov = mv!(MoveType::Line(MoveDir1D::Next));
        vm.input_key(key!(KeyCode::Down));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <Left>
        let mov = mv!(MoveType::Column(MoveDir1D::Previous, false));
        vm.input_key(key!(KeyCode::Left));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <Right>
        let mov = mv!(MoveType::Column(MoveDir1D::Next, false));
        vm.input_key(key!(KeyCode::Right));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <S-Left>
        let mov = mv!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Previous));
        vm.input_key(key!(KeyCode::Left, KeyModifiers::SHIFT));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <S-Right>
        let mov = mv!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Next));
        vm.input_key(key!(KeyCode::Right, KeyModifiers::SHIFT));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <C-Left>
        let mov = mv!(MoveType::WordBegin(WordStyle::Big, MoveDir1D::Previous));
        vm.input_key(key!(KeyCode::Left, KeyModifiers::CONTROL));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <C-Right>
        let mov = mv!(MoveType::WordBegin(WordStyle::Big, MoveDir1D::Next));
        vm.input_key(key!(KeyCode::Right, KeyModifiers::CONTROL));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <End>
        let mov = mv!(MoveType::LinePos(MovePosition::End), Count::MinusOne);
        vm.input_key(key!(KeyCode::End));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <Enter>
        let mov = mv!(MoveType::FirstWord(MoveDir1D::Next));
        vm.input_key(key!(KeyCode::Enter));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // <Home>
        let mov = mv!(MoveType::LinePos(MovePosition::Beginning), 0);
        ctx.persist.shape = None;
        vm.input_key(key!(KeyCode::Home));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_visual_ops() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // Move into Visual mode (charwise)
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Delete with "d"
        let op = EditAction::Delete;
        let mov = Action::Edit(op.into(), EditTarget::Selection);
        vm.input_key(key!('d'));
        assert_pop1!(vm, mov, ctx);

        // We move back to Normal mode after deletion.
        ctx.persist.shape = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Move into Visual mode (charwise)
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Yank with "y"
        let op = EditAction::Yank;
        let mov = Action::Edit(op.into(), EditTarget::Selection);
        vm.input_key(key!('y'));
        assert_pop1!(vm, mov, ctx);

        // We move back to Normal after yanking.
        ctx.persist.shape = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Move into Visual mode (charwise)
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Uppercase with "gu"
        let op = EditAction::ChangeCase(Case::Lower);
        let mov = Action::Edit(op.into(), EditTarget::Selection);
        vm.input_key(key!('g'));
        vm.input_key(key!('u'));
        assert_pop1!(vm, mov, ctx);

        // We move back to Normal mode after changing case.
        ctx.persist.shape = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Move into Visual mode (charwise)
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Uppercase with "gu"
        let op = EditAction::ChangeCase(Case::Upper);
        let mov = Action::Edit(op.into(), EditTarget::Selection);
        vm.input_key(key!('g'));
        vm.input_key(key!('U'));
        assert_pop1!(vm, mov, ctx);

        // Move back to Normal mode after changing case.
        ctx.persist.shape = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Move into Visual mode (charwise)
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Shape made LineWise with "D"
        let op = EditAction::Delete;
        let mov = mvop!(op, MoveType::LinePos(MovePosition::End), 0);
        ctx.persist.shape = Some(TargetShape::LineWise);
        vm.input_key(key!('D'));
        assert_pop1!(vm, SEL_SPLIT, ctx);
        assert_pop1!(vm, BLOCK_BEG, ctx);
        assert_pop1!(vm, mov, ctx);

        // Move back into Normal mode after deletion.
        ctx.persist.shape = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Move into Visual mode (charwise)
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Shape made LineWise with "Y".
        let op = EditAction::Yank;
        let mov = Action::Edit(op.into(), EditTarget::Selection);
        ctx.persist.shape = Some(TargetShape::LineWise);
        vm.input_key(key!('Y'));
        assert_pop1!(vm, mov, ctx);

        // Move back into Normal mode after yanking.
        ctx.persist.shape = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Move into Visual mode (charwise)
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Shape made LineWise with "X"
        let op = EditAction::Delete;
        let mov = Action::Edit(op.into(), EditTarget::Selection);
        ctx.persist.shape = Some(TargetShape::LineWise);
        vm.input_key(key!('X'));
        assert_pop1!(vm, SEL_SPLIT, ctx);
        assert_pop1!(vm, BLOCK_BEG, ctx);
        assert_pop1!(vm, mov, ctx);

        // Move back into Normal mode after deletion.
        ctx.persist.shape = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Move into Visual mode (charwise)
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Shape made LineWise with "R"
        let op = EditAction::Delete;
        let mov = Action::Edit(op.into(), EditTarget::Selection);
        ctx.persist.shape = Some(TargetShape::LineWise);
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('R'));
        assert_pop1!(vm, mov, ctx);

        // Moves into Insert mode after "R".
        ctx.persist.shape = None;
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Move into Normal mode with ^C.
        ctx.persist.shape = None;
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Move into Visual mode (charwise)
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Shape made LineWise with "S"
        let op = EditAction::Delete;
        let mov = Action::Edit(op.into(), EditTarget::Selection);
        ctx.persist.shape = Some(TargetShape::LineWise);
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('S'));
        assert_pop1!(vm, mov, ctx);

        // Move into Insert mode after "S".
        ctx.persist.shape = None;
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Move back into Normal mode.
        ctx.persist.shape = None;
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Move into Visual mode (blockwise)
        ctx.persist.shape = Some(TargetShape::BlockWise);
        vm.input_key(ctl!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Shape remains BlockWise with "X"
        let op = EditAction::Delete;
        let mov = Action::Edit(op.into(), EditTarget::Selection);
        vm.input_key(key!('X'));
        assert_pop1!(vm, SEL_SPLIT, ctx);
        assert_pop1!(vm, BLOCK_BEG, ctx);
        assert_pop1!(vm, mov, ctx);

        // Move back into Normal mode after deletion.
        ctx.persist.shape = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);

        // Move into Visual mode (blockwise)
        ctx.persist.shape = Some(TargetShape::BlockWise);
        vm.input_key(ctl!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Shape remains BlockWise with "Y".
        let op = EditAction::Yank;
        let mov = Action::Edit(op.into(), EditTarget::Selection);
        vm.input_key(key!('Y'));
        assert_pop1!(vm, mov, ctx);

        // Move back into Normal mode after yanking.
        ctx.persist.shape = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, CURRENT_POS, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_visual_block_insert() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // Move into Visual mode (blockwise)
        ctx.persist.shape = Some(TargetShape::BlockWise);
        vm.input_key(ctl!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Insert at beginning of block ("I").
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('I'));
        assert_pop1!(vm, BLOCK_SPLIT, ctx);
        assert_pop1!(vm, BLOCK_BEG, ctx);
        assert_pop1!(vm, CURSOR_SPLIT, ctx);

        // Moves into Insert mode after "I".
        ctx.persist.shape = None;
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Move back into Normal mode.
        ctx.persist.shape = None;
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Move into Visual mode (blockwise)
        ctx.persist.shape = Some(TargetShape::BlockWise);
        vm.input_key(ctl!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Insert at beginning of block ("A").
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('A'));
        assert_pop1!(vm, BLOCK_SPLIT, ctx);
        assert_pop1!(vm, BLOCK_END, ctx);
        assert_pop1!(vm, CURSOR_SPLIT, ctx);

        // Moves into Insert mode after "A".
        ctx.persist.shape = None;
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Move back into Normal mode.
        ctx.persist.shape = None;
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Move into Visual mode (blockwise)
        ctx.persist.shape = Some(TargetShape::BlockWise);
        vm.input_key(ctl!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // Change block ("c").
        ctx.persist.insert = Some(InsertStyle::Insert);
        let del = EditAction::Delete.into();
        let act = Action::Edit(del, EditTarget::Selection);
        vm.input_key(key!('c'));
        assert_pop1!(vm, BLOCK_SPLIT, ctx);
        assert_pop1!(vm, BLOCK_BEG, ctx);
        assert_pop1!(vm, act, ctx);
        assert_pop1!(vm, CURSOR_SPLIT, ctx);

        // Moves into Insert mode after "c".
        ctx.persist.shape = None;
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Move back into Normal mode.
        ctx.persist.shape = None;
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_visual_motion() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // "w" moves forward to the next word beginning
        let mov = mv!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Next));
        ctx.action.count = Some(5);
        vm.input_key(key!('5'));
        vm.input_key(key!('w'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // "e" moves forward to the next word end
        let mov = mv!(MoveType::WordEnd(WordStyle::Little, MoveDir1D::Next));
        ctx.action.count = Some(5);
        vm.input_key(key!('5'));
        vm.input_key(key!('e'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // "W" moves forward to the next WORD beginning
        let mov = mv!(MoveType::WordBegin(WordStyle::Big, MoveDir1D::Next));
        ctx.action.count = Some(10);
        vm.input_key(key!('1'));
        vm.input_key(key!('0'));
        vm.input_key(key!('W'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // "E" moves forward to the next WORD ending
        let mov = mv!(MoveType::WordEnd(WordStyle::Big, MoveDir1D::Next));
        ctx.action.count = Some(10);
        vm.input_key(key!('1'));
        vm.input_key(key!('0'));
        vm.input_key(key!('E'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // "b" moves backward to the previous WORD beginning
        let mov = mv!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Previous));
        ctx.action.count = None;
        vm.input_key(key!('b'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // "B" moves WORD forward
        let mov = mv!(MoveType::WordBegin(WordStyle::Big, MoveDir1D::Previous));
        ctx.action.count = None;
        vm.input_key(key!('B'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // "E" moves forward to the next WORD ending
        let mov = mv!(MoveType::WordEnd(WordStyle::Little, MoveDir1D::Previous));
        ctx.action.count = Some(7);
        vm.input_key(key!('7'));
        vm.input_key(key!('g'));
        vm.input_key(key!('e'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        // "E" moves forward to the next WORD ending
        let mov = mv!(MoveType::WordEnd(WordStyle::Big, MoveDir1D::Previous));
        ctx.action.count = Some(9);
        vm.input_key(key!('9'));
        vm.input_key(key!('g'));
        vm.input_key(key!('E'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);
    }

    #[test]
    fn test_force_motion() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        let mov = mv!(MoveType::WordBegin(WordStyle::Little, MoveDir1D::Next));

        ctx.action.operation = EditAction::Delete;

        // By default, there's no shape in the context.
        ctx.persist.shape = None;
        vm.input_key(key!('d'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // "v" forces charwise
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('d'));
        vm.input_key(key!('v'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);
        assert_eq!(vm.context().persist.shape, None);

        // "V" forces linewise
        ctx.persist.shape = Some(TargetShape::LineWise);
        vm.input_key(key!('d'));
        vm.input_key(key!('V'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);
        assert_eq!(vm.context().persist.shape, None);

        // <C-V> forces blockwise
        ctx.persist.shape = Some(TargetShape::BlockWise);
        vm.input_key(key!('d'));
        vm.input_key(ctl!('v'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);
        assert_eq!(vm.context().persist.shape, None);

        // If multiple force-motion keys are pressed, most recent is used.
        ctx.persist.shape = Some(TargetShape::BlockWise);
        vm.input_key(key!('d'));
        vm.input_key(key!('v'));
        vm.input_key(key!('V'));
        vm.input_key(ctl!('v'));
        vm.input_key(key!('w'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);
        assert_eq!(vm.context().persist.shape, None);
    }

    #[test]
    fn test_insert_mode() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        ctx.persist.insert = Some(InsertStyle::Insert);

        vm.input_key(key!('i'));
        assert_pop2!(vm, CURSOR_SPLIT, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        vm.input_key(key!('i'));
        assert_pop1!(vm, typechar!('i'), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        vm.input_key(key!('I'));
        assert_pop1!(vm, typechar!('I'), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        vm.input_key(key!('d'));
        assert_pop1!(vm, typechar!('d'), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        vm.input_key(key!('C'));
        assert_pop1!(vm, typechar!('C'), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        vm.input_key(key!('$'));
        assert_pop1!(vm, typechar!('$'), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Type a digraph.
        vm.input_key(ctl!('k'));
        assert_eq!(vm.get_cursor_indicator(), Some('?'));

        vm.input_key(key!('L'));
        assert_eq!(vm.get_cursor_indicator(), Some('L'));

        ctx.action.cursor = Some('L');
        ctx.ch.digraph1 = Some('L');
        ctx.ch.digraph2 = Some('i');
        vm.input_key(key!('i'));
        assert_pop1!(vm, TYPE_CONTEXTUAL, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Type a literal.
        vm.input_key(ctl!('v'));
        assert_eq!(vm.get_cursor_indicator(), Some('^'));

        ctx.action.cursor = Some('^');
        ctx.ch.digraph1 = None;
        ctx.ch.digraph2 = None;
        ctx.ch.any = Some(ctl!('g').into());
        vm.input_key(ctl!('g'));
        assert_pop1!(vm, TYPE_CONTEXTUAL, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Type a codepoint.
        ctx.action.cursor = Some('^');
        ctx.ch.oct = Some(97);
        ctx.ch.any = None;
        vm.input_key(ctl!('v'));
        vm.input_key(key!('o'));
        vm.input_key(key!('1'));
        vm.input_key(key!('4'));
        vm.input_key(key!('1'));
        assert_pop1!(vm, TYPE_CONTEXTUAL, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        ctx.action.cursor = None;
        ctx.ch.oct = None;

        // Enter Replace mode by pressing <Ins>.
        ctx.persist.insert = Some(InsertStyle::Replace);
        vm.input_key(key!(KeyCode::Insert));
        assert_pop2!(vm, CURSOR_SPLIT, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        vm.input_key(key!('d'));
        assert_pop1!(vm, typechar!('d'), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        let mov =
            mvop!(EditAction::Motion, MoveType::WordBegin(WordStyle::Little, MoveDir1D::Previous));

        vm.input_key(key!(KeyCode::Left, KeyModifiers::SHIFT));
        assert_pop1!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        let mov =
            mvop!(EditAction::Motion, MoveType::WordBegin(WordStyle::Little, MoveDir1D::Next));

        vm.input_key(key!(KeyCode::Right, KeyModifiers::SHIFT));
        assert_pop1!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        let it = InsertTextAction::Paste(MoveDir1D::Previous, Count::Exact(1));
        ctx.action.cursor = Some('"');
        ctx.action.register = Some(Register::Named('z'));
        ctx.action.register_append = false;
        vm.input_key(ctl!('r'));
        assert_eq!(vm.pop(), None);
        vm.input_key(key!('z'));
        assert_pop1!(vm, Action::from(it), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Pressing ^R^C should go back to Normal mode.
        ctx.action.cursor = Some('"');
        ctx.action.register = None;
        ctx.persist.insert = None;
        vm.input_key(ctl!('r'));
        assert_eq!(vm.pop(), None);
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_pop1!(vm, CHECKPOINT, ctx);
        assert_eq!(vm.mode(), VimMode::Normal);
    }

    #[test]
    fn test_override() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // Check the original Normal mode mapping.
        let mov = mv!(MoveType::ScreenLine(MoveDir1D::Next));
        vm.input_key(key!('g'));
        vm.input_key(key!('j'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // Override "gj" so that it doesn't do screen line movement.
        let step = edit_end!(MoveType::Line(MoveDir1D::Next));
        add_mapping(&mut vm, &NMAP, "gj", &step);

        // Normal mode "gj" should be overridden now.
        let mov = mv!(MoveType::Line(MoveDir1D::Next));
        vm.input_key(key!('g'));
        vm.input_key(key!('j'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // Other Normal mode mappings beginning with "g" should still work.
        let mov = mv!(MoveType::ScreenLine(MoveDir1D::Previous));
        vm.input_key(key!('g'));
        vm.input_key(key!('k'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        let mov = mv!(MoveType::ScreenLinePos(MovePosition::Beginning), 0);
        vm.input_key(key!('g'));
        vm.input_key(key!('0'));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // Visual mode "gj" should still be the original mapping.
        let mov = mv!(MoveType::ScreenLine(MoveDir1D::Next));
        ctx.persist.shape = Some(TargetShape::CharWise);
        vm.input_key(key!('v'));
        assert_pop2!(vm, CURRENT_POS, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);

        vm.input_key(key!('g'));
        vm.input_key(key!('j'));
        assert_pop2!(vm, mov, ctx);
        assert_eq!(vm.mode(), VimMode::Visual);
    }

    #[test]
    fn test_count_alters_motion() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // Without a count, "%" is ItemMatch.
        let mot = mv!(MoveType::ItemMatch);

        vm.input_key(key!('%'));
        assert_pop1!(vm, mot, ctx);
        assert_normal!(vm, ctx);

        ctx.action.operation = EditAction::Delete;
        vm.input_key(key!('d'));
        vm.input_key(key!('%'));
        assert_pop1!(vm, mot, ctx);
        assert_normal!(vm, ctx);

        // With a count, "%" becomes BufferLinePercent.
        let mot = mv!(MoveType::BufferLinePercent);

        ctx.action.count = Some(1);
        ctx.action.operation = EditAction::Motion;
        vm.input_key(key!('1'));
        vm.input_key(key!('%'));
        assert_pop1!(vm, mot, ctx);
        assert_normal!(vm, ctx);

        ctx.action.count = Some(88);
        ctx.action.operation = EditAction::Yank;
        vm.input_key(key!('8'));
        vm.input_key(key!('8'));
        vm.input_key(key!('y'));
        vm.input_key(key!('%'));
        assert_pop1!(vm, mot, ctx);
        assert_normal!(vm, ctx);

        ctx.action.count = Some(101);
        ctx.action.operation = EditAction::Delete;
        vm.input_key(key!('d'));
        vm.input_key(key!('1'));
        vm.input_key(key!('0'));
        vm.input_key(key!('1'));
        vm.input_key(key!('%'));
        assert_pop1!(vm, mot, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_count_alters_window() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // Without a count, ^Wo closes all windows besides the currently focused one.
        let target = CloseTarget::AllBut(FocusChange::Current);
        let act: Action = WindowAction::Close(target, CloseFlags::QUIT).into();

        ctx.action.count = None;
        vm.input_key(ctl!('w'));
        vm.input_key(key!('o'));
        assert_pop1!(vm, act, ctx);
        assert_normal!(vm, ctx);

        ctx.action.count = None;
        vm.input_key(ctl!('w'));
        vm.input_key(ctl!('o'));
        assert_pop1!(vm, act, ctx);
        assert_normal!(vm, ctx);

        // With a count ^Wo closes all but the specified window.
        let target = CloseTarget::AllBut(FocusChange::Offset(Count::Contextual, true));
        let act: Action = WindowAction::Close(target, CloseFlags::QUIT).into();

        ctx.action.count = Some(5);
        vm.input_key(key!('5'));
        vm.input_key(ctl!('w'));
        vm.input_key(key!('o'));
        assert_pop1!(vm, act, ctx);
        assert_normal!(vm, ctx);

        ctx.action.count = Some(8);
        vm.input_key(key!('8'));
        vm.input_key(ctl!('w'));
        vm.input_key(ctl!('o'));
        assert_pop1!(vm, act, ctx);
        assert_normal!(vm, ctx);

        // Without a count ^Wv splits the window.
        let act: Action =
            WindowAction::Split(Axis::Vertical, MoveDir1D::Previous, Count::Contextual).into();
        ctx.action.count = None;
        vm.input_key(ctl!('w'));
        vm.input_key(key!('v'));
        assert_pop1!(vm, act, ctx);
        assert_normal!(vm, ctx);

        // With a count ^Wv splits the window and resizes it.
        let acts: Action =
            WindowAction::Split(Axis::Vertical, MoveDir1D::Previous, Count::Exact(1)).into();
        let actr: Action =
            WindowAction::Resize(Axis::Vertical, SizeChange::Exact(Count::Contextual)).into();
        ctx.action.count = Some(10);
        vm.input_key(key!('1'));
        vm.input_key(key!('0'));
        vm.input_key(ctl!('w'));
        vm.input_key(key!('v'));
        assert_pop1!(vm, acts, ctx);
        assert_pop1!(vm, actr, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_scrollcp() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // Place cursored line at the top of the screen with "zt".
        let act = Action::Scroll(ScrollStyle::CursorPos(MovePosition::Beginning, Axis::Vertical));
        vm.input_key(key!('z'));
        vm.input_key(key!('t'));
        assert_pop1!(vm, act, ctx);
        assert_normal!(vm, ctx);

        // Adding a count to "zt" makes it go to a specific line.
        let act = Action::Scroll(ScrollStyle::LinePos(MovePosition::Beginning, Count::Contextual));
        ctx.action.count = Some(2118);
        vm.input_key(key!('2'));
        vm.input_key(key!('1'));
        vm.input_key(key!('1'));
        vm.input_key(key!('8'));
        vm.input_key(key!('z'));
        vm.input_key(key!('t'));
        assert_pop1!(vm, act, ctx);
        assert_normal!(vm, ctx);

        // "z<Enter>" works like "zt", but it also goes to the first word on the line.
        let actfw = mvop!(EditAction::Motion, MoveType::FirstWord(MoveDir1D::Next), 0);
        let actcp = Action::Scroll(ScrollStyle::CursorPos(MovePosition::Beginning, Axis::Vertical));
        ctx.action.count = None;
        vm.input_key(key!('z'));
        vm.input_key(key!(KeyCode::Enter));
        assert_pop1!(vm, actfw, ctx);
        assert_pop1!(vm, actcp, ctx);
        assert_normal!(vm, ctx);

        // Like with "zt", giving "z<Enter>" a count goes to that line.
        let actfw = mvop!(EditAction::Motion, MoveType::FirstWord(MoveDir1D::Next), 0);
        let actlp =
            Action::Scroll(ScrollStyle::LinePos(MovePosition::Beginning, Count::Contextual));
        ctx.action.count = Some(1312);
        vm.input_key(key!('1'));
        vm.input_key(key!('3'));
        vm.input_key(key!('1'));
        vm.input_key(key!('2'));
        vm.input_key(key!('z'));
        vm.input_key(key!(KeyCode::Enter));
        assert_pop1!(vm, actfw, ctx);
        assert_pop1!(vm, actlp, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_literal() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('i'));
        assert_pop2!(vm, CURSOR_SPLIT, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Test that cursor indicator gets set to '^' while we're typing.
        vm.input_key(ctl!('v'));
        assert_eq!(vm.pop(), None);
        assert_eq!(vm.mode(), VimMode::Insert);
        assert_eq!(vm.get_cursor_indicator(), Some('^'));

        ctx.action.cursor = Some('^');
        ctx.ch.any = Some(key!(KeyCode::Esc).into());
        vm.input_key(key!(KeyCode::Esc));
        assert_pop2!(vm, TYPE_CONTEXTUAL, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);
        assert_eq!(vm.get_cursor_indicator(), None);

        // Test that typing in a full octal sequence works.
        ctx.action.cursor = Some('^');
        ctx.ch.oct = Some(127);
        ctx.ch.any = None;
        vm.input_key(ctl!('v'));
        vm.input_key(key!('o'));
        vm.input_key(key!('1'));
        vm.input_key(key!('7'));
        vm.input_key(key!('7'));
        assert_pop2!(vm, TYPE_CONTEXTUAL, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Test that valid octal character types normally afterwards.
        ctx.action.cursor = None;
        ctx.ch.oct = None;
        vm.input_key(key!('7'));
        assert_pop2!(vm, typechar!('7'), ctx);

        // Test that typing in an incomplete octal sequence works.
        vm.input_key(ctl!('v'));
        vm.input_key(key!('o'));
        vm.input_key(key!('7'));
        vm.input_key(key!('8'));

        ctx.action.cursor = Some('^');
        ctx.ch.oct = Some(7);
        assert_pop1!(vm, TYPE_CONTEXTUAL, ctx);

        ctx.action.cursor = None;
        ctx.ch.oct = None;
        assert_pop2!(vm, typechar!('8'), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Test that typing in a decimal sequence works.
        vm.input_key(ctl!('v'));
        vm.input_key(key!('1'));
        vm.input_key(key!('2'));
        vm.input_key(key!('3'));
        vm.input_key(key!('4'));

        ctx.action.cursor = Some('^');
        ctx.ch.dec = Some(123);
        assert_pop1!(vm, TYPE_CONTEXTUAL, ctx);

        ctx.action.cursor = None;
        ctx.ch.dec = None;
        assert_pop2!(vm, typechar!('4'), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Test that typing in a hexadecimal sequence works.
        vm.input_key(ctl!('v'));
        vm.input_key(key!('u'));
        vm.input_key(key!('2'));
        vm.input_key(key!('6'));
        vm.input_key(key!('0'));
        vm.input_key(key!('3'));
        vm.input_key(key!('3'));

        ctx.action.cursor = Some('^');
        ctx.ch.hex = Some(9731);
        assert_pop1!(vm, TYPE_CONTEXTUAL, ctx);

        ctx.action.cursor = None;
        ctx.ch.hex = None;
        assert_pop2!(vm, typechar!('3'), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Test that typing in a full lowercase hexadecimal sequence works.
        vm.input_key(ctl!('v'));
        vm.input_key(key!('U'));
        vm.input_key(key!('0'));
        vm.input_key(key!('0'));
        vm.input_key(key!('0'));
        vm.input_key(key!('1'));
        vm.input_key(key!('f'));
        vm.input_key(key!('7'));
        vm.input_key(key!('5'));
        vm.input_key(key!('e'));
        vm.input_key(key!('a'));

        ctx.action.cursor = Some('^');
        ctx.ch.hex = Some(128862);
        assert_pop1!(vm, TYPE_CONTEXTUAL, ctx);

        ctx.action.cursor = None;
        ctx.ch.hex = None;
        assert_pop2!(vm, typechar!('a'), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Test that typing in an incomplete uppercase hexadecimal sequence works.
        vm.input_key(ctl!('v'));
        vm.input_key(key!('U'));
        vm.input_key(key!('1'));
        vm.input_key(key!('F'));
        vm.input_key(key!('4'));
        vm.input_key(key!('6'));
        vm.input_key(key!('D'));
        vm.input_key(key!('G'));

        ctx.action.cursor = Some('^');
        ctx.ch.hex = Some(128109);
        assert_pop1!(vm, TYPE_CONTEXTUAL, ctx);

        ctx.action.cursor = None;
        ctx.ch.hex = None;
        assert_pop2!(vm, typechar!('G'), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);
    }

    #[test]
    fn test_unmapped_reset() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        /*
         * The key "z" is not mapped in Operator Pending mode, so the action context should be
         * reset when it's pressed, causing "l" to be interpreted as a movement.
         */
        let mov = mv!(MoveType::Column(MoveDir1D::Next, false));
        ctx.action.operation = EditAction::Motion;
        vm.input_key(key!('c'));
        vm.input_key(key!('z'));
        vm.input_key(key!('l'));
        assert_pop1!(vm, CHECKPOINT, ctx);
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_count_nullifies() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        // Without a count, Delete deletes one character.
        let op = EditAction::Delete;
        let mov = MoveType::Column(MoveDir1D::Next, false);
        let mov = EditTarget::Motion(mov, Count::Contextual);
        let mov = Action::Edit(op.into(), mov);
        vm.input_key(key!(KeyCode::Delete));
        assert_pop1!(vm, mov, ctx);
        assert_normal!(vm, ctx);

        // With a count, Delete does nothing.
        ctx.action.count = Some(1);
        ctx.action.operation = EditAction::Motion;
        vm.input_key(key!('1'));
        vm.input_key(key!(KeyCode::Delete));
        assert_pop1!(vm, Action::NoOp, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_macro_toggle() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        let toggle = Action::from(MacroAction::ToggleRecording);

        // The first "q" does nothing.
        vm.input_key(key!('q'));
        assert_eq!(vm.pop(), None);
        assert_eq!(vm.context(), ctx);

        // The second "q" is treated as a register name, and recording starts.
        ctx.persist.recording = Some((Register::Named('q'), false));
        vm.input_key(key!('q'));

        ctx.action.register = Some(Register::Named('q'));
        assert_pop1!(vm, toggle, ctx);

        ctx.action.register = None;
        assert_normal!(vm, ctx);

        // Type "gqq" to format.
        let format = rangeop!(EditAction::Format, RangeType::Line);
        ctx.action.operation = EditAction::Format.into();
        vm.input_key(key!('g'));
        vm.input_key(key!('q'));
        vm.input_key(key!('q'));
        assert_pop1!(vm, format, ctx);
        assert_normal!(vm, ctx);

        // Type "q" to end recording.
        ctx.action.operation = EditAction::Motion;
        ctx.persist.recording = None;
        vm.input_key(key!('q'));
        assert_pop1!(vm, toggle, ctx);
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_edit_repeat() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        let col = MoveType::Column(MoveDir1D::Next, false);

        // Feed in a tracked sequence.
        ctx.action.operation = EditAction::Delete;
        ctx.action.count = Some(2);
        vm.input_key(key!('2'));
        vm.input_key(key!('d'));
        vm.input_key(key!('l'));
        assert_pop1!(vm, mv!(col), ctx);
        assert_normal!(vm, ctx);

        // Feed in a breaking sequence.
        ctx.action.operation = EditAction::Motion;
        ctx.action.count = Some(3);
        vm.input_key(key!('3'));
        vm.input_key(key!('l'));
        assert_pop1!(vm, mv!(col), ctx);
        assert_normal!(vm, ctx);

        // Press ".".
        ctx.action.operation = EditAction::Motion;
        ctx.action.count = None;
        vm.input_key(key!('.'));
        assert_pop1!(vm, Action::Repeat(RepeatType::EditSequence), ctx);

        // Check that repeating does 2dw action.
        ctx.action.operation = EditAction::Delete;
        ctx.action.count = Some(2);
        vm.repeat(RepeatType::EditSequence, None);
        assert_pop1!(vm, mv!(col), ctx);

        // Read the Checkpoint from pressing "." earlier.
        assert_normal!(vm, ctx);

        // Press ".".
        ctx.action.operation = EditAction::Motion;
        ctx.action.count = None;
        vm.input_key(key!('.'));
        assert_pop1!(vm, Action::Repeat(RepeatType::EditSequence), ctx);

        // Check that we can override the count and register.
        ctx.action.operation = EditAction::Delete;
        ctx.action.count = Some(4);
        ctx.action.register = Some(Register::Named('a'));
        vm.repeat(RepeatType::EditSequence, Some(ctx.clone()));
        assert_pop1!(vm, mv!(col), ctx);

        // Read the Checkpoint from pressing "." earlier.
        assert_normal!(vm, ctx);

        // Press ".".
        ctx.action.operation = EditAction::Motion;
        ctx.action.count = None;
        ctx.action.register = None;
        vm.input_key(key!('.'));
        assert_pop1!(vm, Action::Repeat(RepeatType::EditSequence), ctx);

        // Repeating without a context now uses the overriden context.
        ctx.action.operation = EditAction::Delete;
        ctx.action.count = Some(4);
        ctx.action.register = Some(Register::Named('a'));
        vm.repeat(RepeatType::EditSequence, None);
        assert_pop1!(vm, mv!(col), ctx);

        // Read the Checkpoint from pressing "." earlier.
        assert_normal!(vm, ctx);

        // Feed in a change sequence.
        ctx.action.count = Some(2);
        ctx.action.register = None;
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('2'));
        vm.input_key(key!('c'));
        vm.input_key(key!('l'));
        assert_pop2!(vm, mv!(col), ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Type some new characters.
        let ch_2 = typechar!('2');
        let ch_c = typechar!('c');
        let ch_l = typechar!('l');

        ctx.action.operation = EditAction::Motion;
        ctx.action.count = None;
        vm.input_key(key!('2'));
        vm.input_key(key!('c'));
        vm.input_key(key!('l'));

        assert_pop1!(vm, ch_2.clone(), ctx);
        assert_pop1!(vm, ch_c.clone(), ctx);
        assert_pop1!(vm, ch_l.clone(), ctx);

        // Back to Normal mode.
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Press ".".
        vm.input_key(key!('.'));
        assert_pop1!(vm, Action::Repeat(RepeatType::EditSequence), ctx);

        // Repeat the whole change sequence.
        vm.repeat(RepeatType::EditSequence, None);

        ctx.action.operation = EditAction::Delete;
        ctx.action.count = Some(2);
        ctx.persist.insert = Some(InsertStyle::Insert);
        assert_pop1!(vm, mv!(col), ctx);

        ctx.action.operation = EditAction::Motion;
        ctx.action.count = None;
        assert_pop1!(vm, ch_2.clone(), ctx);
        assert_pop1!(vm, ch_c.clone(), ctx);
        assert_pop1!(vm, ch_l.clone(), ctx);

        ctx.persist.insert = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);

        // Read the Checkpoint from pressing "." earlier.
        assert_normal!(vm, ctx);
    }

    #[test]
    fn test_edit_repeat_append_line() {
        let mut vm: VimMachine<TerminalKey> = VimMachine::default();
        let mut ctx = VimContext::default();

        let op = EditAction::Motion;
        let mov = mvop!(op, MoveType::LinePos(MovePosition::End), 0);

        // Move down a line, so we do SequenceStatus::Break.
        vm.input_key(key!('j'));
        assert_pop1!(vm, mv!(MoveType::Line(MoveDir1D::Next)), ctx);
        assert_normal!(vm, ctx);

        // Move to Insert mode at end of line.
        ctx.persist.insert = Some(InsertStyle::Insert);
        vm.input_key(key!('A'));
        assert_pop1!(vm, mov, ctx);
        assert_pop2!(vm, CURSOR_SPLIT, ctx);
        assert_eq!(vm.mode(), VimMode::Insert);

        // Type some characters.
        vm.input_key(key!('a'));
        assert_pop1!(vm, typechar!('a'), ctx);

        vm.input_key(key!('b'));
        assert_pop1!(vm, typechar!('b'), ctx);

        vm.input_key(key!('c'));
        assert_pop1!(vm, typechar!('c'), ctx);

        // Back to Normal mode.
        ctx.persist.insert = None;
        vm.input_key(ctl!('c'));
        assert_pop1!(vm, CURSOR_CLOSE, ctx);
        assert_pop1!(vm, COLUMN_PREV, ctx);
        assert_normal!(vm, ctx);

        // Move down a line.
        vm.input_key(key!('j'));
        assert_pop1!(vm, mv!(MoveType::Line(MoveDir1D::Next)), ctx);
        assert_normal!(vm, ctx);

        // Press ".".
        vm.input_key(key!('.'));
        assert_pop1!(vm, Action::Repeat(RepeatType::EditSequence), ctx);

        // Repeat the whole append sequence, including moving to EOL.
        vm.repeat(RepeatType::EditSequence, Some(ctx.clone()));

        ctx.persist.insert = Some(InsertStyle::Insert);
        assert_pop1!(vm, mov, ctx);
        assert_pop1!(vm, CURSOR_SPLIT, ctx);
        assert_pop1!(vm, typechar!('a'), ctx);
        assert_pop1!(vm, typechar!('b'), ctx);
        assert_pop1!(vm, typechar!('c'), ctx);

        ctx.persist.insert = None;
        assert_pop1!(vm, CURSOR_CLOSE, ctx);

        // Read the Checkpoint from pressing "." earlier.
        assert_normal!(vm, ctx);
    }
}