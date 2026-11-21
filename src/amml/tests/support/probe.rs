use std::sync::{Arc, Mutex};

use amml_api::amml_abi::ClientTick;
use amml_api::testing::{FakeClient, FakeGame};
use amml_api::{Client, HookOrder, Listener};

// runs `read` with the client of a tick of `game` serving `state`, and gives what it returned
pub struct Probe;

impl Probe {
    pub fn read<R: Send + 'static>(
        game: &FakeGame,
        state: FakeClient,
        read: impl Fn(&Client<'_>) -> R + Send + 'static,
    ) -> R {
        game.offer::<ClientTick>();
        game.set_client(state);
        let result = Arc::new(Mutex::new(None));
        let slot = Arc::clone(&result);
        let listener: Listener = game
            .ctx()
            .events()
            .listen::<ClientTick>(HookOrder::NORMAL, move |_, at| {
                if let Some(client) = at.client() {
                    *slot.lock().unwrap() = Some(read(&client));
                }
            })
            .unwrap();
        game.raise_with_client(&mut ClientTick { header: FakeGame::header::<ClientTick>() });
        drop(listener);
        let value = result.lock().unwrap().take();
        value.expect("the probe ran inside a tick with a client")
    }
}

// runs a drawer (or any code that needs the canvas of a frame) inside the `WorldRendered` of a fake game
pub struct RenderProbe;

impl RenderProbe {
    // raises one `WorldRendered` for `canvas`; `run` gets the canvas of that frame
    pub fn frame(
        game: &FakeGame,
        canvas: &amml_api::testing::FakeCanvas,
        run: impl FnMut(&amml_api::WorldCanvas<'_>) + Send + 'static,
    ) {
        use amml_api::WorldRenderedExt;
        let run = Mutex::new(run);
        let listener = game
            .ctx()
            .events()
            .listen::<amml_api::amml_abi::WorldRendered>(HookOrder::NORMAL, move |event, _| {
                let canvas = event.canvas().expect("a canvas inside the dispatch");
                (run.lock().unwrap())(&canvas);
            })
            .unwrap();
        game.raise_world_rendered(canvas);
        drop(listener);
    }
}
