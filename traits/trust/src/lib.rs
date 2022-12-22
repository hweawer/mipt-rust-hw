#![forbid(unsafe_code)]

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundOutcome {
    BothCooperated,
    LeftCheated,
    RightCheated,
    BothCheated,
}

pub struct Game {
    left: Box<dyn GameActor>,
    right: Box<dyn GameActor>,
    left_score: i32,
    right_score: i32,
}

impl Game {
    pub fn new(left: Box<dyn GameActor>, right: Box<dyn GameActor>) -> Self {
        Self {
            left,
            right,
            left_score: 0,
            right_score: 0,
        }
    }

    pub fn left_score(&self) -> i32 {
        self.left_score
    }

    pub fn right_score(&self) -> i32 {
        self.right_score
    }

    pub fn play_round(&mut self) -> RoundOutcome {
        let left_action = self.left.act();
        let right_action = self.right.act();

        let res = match (&left_action, &right_action) {
            (Action::Cheat, Action::Cheat) => RoundOutcome::BothCheated,
            (Action::Cooperate, Action::Cooperate) => {
                self.left_score += 2;
                self.right_score += 2;
                RoundOutcome::BothCooperated
            }
            (Action::Cooperate, Action::Cheat) => {
                self.left_score -= 1;
                self.right_score += 3;
                RoundOutcome::RightCheated
            }
            (Action::Cheat, Action::Cooperate) => {
                self.left_score += 3;
                self.right_score -= 1;
                RoundOutcome::LeftCheated
            }
        };
        self.left.react(right_action);
        self.right.react(left_action);
        res
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct CheatingAgent {}

impl GameActor for CheatingAgent {
    fn act(&mut self) -> Action {
        Action::Cheat
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Default)]
pub struct CooperatingAgent {}

impl GameActor for CooperatingAgent {
    fn act(&mut self) -> Action {
        Action::Cooperate
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct GrudgerAgent {
    pub was_betrayed: bool,
}

impl GameActor for GrudgerAgent {
    fn act(&mut self) -> Action {
        if self.was_betrayed {
            Action::Cheat
        } else {
            Action::Cooperate
        }
    }

    fn react(&mut self, other_action: Action) {
        if other_action == Action::Cheat {
            self.was_betrayed = true;
        }
    }
}

impl Default for GrudgerAgent {
    fn default() -> Self {
        Self {
            was_betrayed: false,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////

pub struct CopycatAgent {
    pub op_last_turn: Option<Action>,
}

impl Default for CopycatAgent {
    fn default() -> Self {
        Self { op_last_turn: None }
    }
}

impl GameActor for CopycatAgent {
    fn act(&mut self) -> Action {
        self.op_last_turn.clone().unwrap_or(Action::Cooperate)
    }

    fn react(&mut self, other_action: Action) {
        self.op_last_turn = Some(other_action)
    }
}

////////////////////////////////////////////////////////////////////////////////
//  "cooperate", "cheat", "cooperate", "cooperate"
pub struct DetectiveAgent {
    step: u64,
    op_cheated: bool,
    op_last_turn: Option<Action>,
}

impl GameActor for DetectiveAgent {
    fn act(&mut self) -> Action {
        if self.step == 0 || self.step == 2 || self.step == 3 {
            Action::Cooperate
        } else if self.step == 1 || !self.op_cheated {
            Action::Cheat
        } else {
            self.op_last_turn.clone().unwrap_or(Action::Cooperate)
        }
    }

    fn react(&mut self, other_action: Action) {
        self.step += 1;
        if other_action == Action::Cheat {
            self.op_cheated = true;
        }
        self.op_last_turn = Some(other_action);
    }
}

impl Default for DetectiveAgent {
    fn default() -> Self {
        Self {
            step: 0,
            op_cheated: false,
            op_last_turn: None,
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub enum Action {
    Cheat,
    Cooperate,
}

pub trait GameActor {
    fn act(&mut self) -> Action;

    fn react(&mut self, other_action: Action) {}
}
