use crate::{
    modules::HEADING_SIGNAL,
    strategy::{Data, COORDINATE_SIGNAL, FIELD_LENGTH, FIELD_MARGIN_Y},
};

#[derive(Default)]
pub struct GetOutState {}

pub async fn run(data: Data, _: &mut GetOutState) {
    let (x, y, _) = data.coordinates;

    HEADING_SIGNAL.signal(0.);

    if y < FIELD_LENGTH / 2. {
        COORDINATE_SIGNAL.signal((x, FIELD_MARGIN_Y));
    } else {
        COORDINATE_SIGNAL.signal((x, FIELD_LENGTH - FIELD_MARGIN_Y));
    }
}
