use axelwas_chess::{Color, EndStates, Move, Piece, PieceTypes, Place, Position};

pub struct Game {
    pub position: Position,
    pub legal: Vec<Move>,
}

pub const PROMOTIONS_CHOICESAA: [PieceTypes; 4] =
    [
    PieceTypes::Queen,
    PieceTypes::Rook,
    PieceTypes::Bishop,
    PieceTypes::Knight
    ];

impl Game {
    pub fn new() -> Game {
        let position = Position::default();
        let legal = position.all_moves();

        Game {
            position,
            legal,
        }
    }

    pub fn piece_at(&self, place: Place) -> Option<&Piece> {
        return self.position.piece_on(place)
    }

    pub fn turn(&self) -> Color {
        return self.position.turn
    }

    pub fn is_legal(&self, from: Place, to: Place) -> bool {
        let played_move = from.goto(&to);
        return self.legal.contains(&played_move) || PROMOTIONS_CHOICESAA.iter().any(|t| self.legal.contains(&played_move.into_promotion(Some(*t))))
    }

    // true when a pawn is on the backrank so ui has to ask
    pub fn is_promotion(&self, from: Place, to: Place) -> bool {
        let is_pawn = matches!(
            self.piece_at(from).map(|p| p.piece_type),
            Some(PieceTypes::Pawn { .. })
        );
        return is_pawn && (to.row == 0 || to.row == 7)
    }

    pub fn legal_moves(&self, from: Place) -> Vec<Place> {
        let mut targets = Vec::new();
        for row in 0..8 {
            for file in 0..8 {
                let to = Place {
                    row,
                    file
                };
                if self.is_legal(from, to) {
                    targets.push(to);
                }
            }
        }
        return targets
    }

    fn build_move(&self, from: Place, to: Place, promote_to: Option<PieceTypes>) -> Option<Move> {
        let wanttoplay = if self.is_promotion(from, to) {
            from.goto(&to).into_promotion(Some(promote_to.unwrap_or(PieceTypes::Queen)))
        } else {
            from.goto(&to)
        };
        if !self.legal.contains(&wanttoplay) {
            return None;
        }
        return Some(wanttoplay)
    }

    pub fn board_previewing(&self, from: Place, to: Place, promote_to: Option<PieceTypes>) -> Option<String> {
        let mut board = String::with_capacity(64);
        let mv = self.build_move(from, to, promote_to)?;
        let next = self.position.clone().execute_move(mv).ok()?;

        for row in 0..8 {
            for file in 0..8 {
                let piece = next.piece_on(Place{row, file});

                if piece.is_some() {
                    board.push(piece.unwrap().into_ascii());
                    println!("{:?}", piece.unwrap().into_ascii());
                } else {
                    board.push('\x20');
                }
            }
        }

        return Some(board)
    }

    pub fn play(&mut self, from: Place, to: Place, promote_to: Option<PieceTypes>) -> bool {
        let wanttoplay = match self.build_move(from, to, promote_to) {
            Some(m) => m,
            None => return false,
        };

        let next = match
            self.position.clone().execute_move(wanttoplay) {
            Ok(position) => position,
            Err(_) => return false,
        };

        self.position = next;
        self.legal = self.position.all_moves();
        return true
    }

    pub fn status(&self) -> EndStates {
        if !self.legal.is_empty() {
            EndStates::None
        } else if self.position.in_check(self.position.turn) {
            EndStates::Checkmate
        } else {
            EndStates::Stalemate
        }
    }

}
