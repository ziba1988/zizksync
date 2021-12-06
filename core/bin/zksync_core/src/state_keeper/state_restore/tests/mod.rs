use zksync_types::{BlockNumber, TokenId};

use self::state_generator::StateGenerator;
use crate::state_keeper::state_restore::tree_restore::RestoredTree;

mod state_generator;

fn generate_blocks(generator: &mut StateGenerator, blocks: usize, cache_on: Option<BlockNumber>) {
    let accounts: Vec<_> = (0..20).map(|_| generator.create_account()).collect();
    for block in 1..=blocks {
        for account in &accounts {
            generator.change_account_balance(*account, TokenId(account.0), 100u64);
        }
        let current_block = BlockNumber(block as u32);
        if Some(current_block) == cache_on {
            generator.save_cache(current_block);
        }

        generator.seal_block();
    }
}

#[tokio::test]
async fn no_cache_restore() {
    const N_BLOCKS: usize = 3;
    const LAST_BLOCK: BlockNumber = BlockNumber(N_BLOCKS as u32);

    let mut state_generator = StateGenerator::new();
    generate_blocks(&mut state_generator, N_BLOCKS, None);

    let mut db = state_generator.create_db();
    assert_eq!(db.load_last_committed_block().await, LAST_BLOCK);
    assert_eq!(db.load_last_cached_block().await, None);

    let mut restorer = RestoredTree::new(db.into());
    restorer.restore().await;

    // Check that root hash is actually restored.
    assert_eq!(restorer.tree.root_hash(), state_generator.tree.root_hash());
}

#[tokio::test]
async fn cached_state_restore_last_block() {
    const N_BLOCKS: usize = 3;
    const LAST_BLOCK: BlockNumber = BlockNumber(N_BLOCKS as u32);

    let mut state_generator = StateGenerator::new();
    generate_blocks(&mut state_generator, N_BLOCKS, Some(LAST_BLOCK));

    let mut db = state_generator.create_db();
    assert_eq!(db.load_last_committed_block().await, LAST_BLOCK);
    assert_eq!(db.load_last_cached_block().await, Some(LAST_BLOCK));

    let mut restorer = RestoredTree::new(db.into());
    restorer.restore().await;

    // Check that root hash is actually restored.
    assert_eq!(restorer.tree.root_hash(), state_generator.tree.root_hash());
}

#[tokio::test]
async fn cached_state_restore_previous_block() {
    const N_BLOCKS: usize = 3;
    const LAST_BLOCK: BlockNumber = BlockNumber(N_BLOCKS as u32);

    let mut state_generator = StateGenerator::new();
    generate_blocks(&mut state_generator, N_BLOCKS, Some(LAST_BLOCK - 1));

    let mut db = state_generator.create_db();
    assert_eq!(db.load_last_committed_block().await, LAST_BLOCK);
    assert_eq!(db.load_last_cached_block().await, Some(LAST_BLOCK - 1));

    let mut restorer = RestoredTree::new(db.into());
    restorer.restore().await;

    // Check that root hash is actually restored.
    assert_eq!(restorer.tree.root_hash(), state_generator.tree.root_hash());
}

#[tokio::test]
#[should_panic(expected = "Root hashes diverged. \n Block 3.")]
async fn no_cache_wrong_root() {
    const N_BLOCKS: usize = 3;
    const LAST_BLOCK: BlockNumber = BlockNumber(N_BLOCKS as u32);

    let mut state_generator = StateGenerator::new();
    generate_blocks(&mut state_generator, N_BLOCKS, None);

    let mut db = state_generator.create_db();
    assert_eq!(db.load_last_committed_block().await, LAST_BLOCK);
    assert_eq!(db.load_last_cached_block().await, None);

    // Set the wrong root hash.
    // Restoring must panic.
    db.set_block_root_hash(LAST_BLOCK, Default::default());

    let mut restorer = RestoredTree::new(db.into());
    restorer.restore().await;
}

#[tokio::test]
#[should_panic(expected = "Root hashes diverged. \n Block 2.")]
async fn no_cache_wrong_root_previous() {
    const N_BLOCKS: usize = 3;
    const LAST_BLOCK: BlockNumber = BlockNumber(N_BLOCKS as u32);

    let mut state_generator = StateGenerator::new();
    generate_blocks(&mut state_generator, N_BLOCKS, None);

    let mut db = state_generator.create_db();
    assert_eq!(db.load_last_committed_block().await, LAST_BLOCK);
    assert_eq!(db.load_last_cached_block().await, None);

    // Here we set two blocks with the wrong root hash: the last and the previous.
    // Last must be set, as initially we only check for the latest root.
    // Previous is set to check that restoring finds the block where hashes diverged correctly.
    // Restoring must panic.
    db.set_block_root_hash(LAST_BLOCK - 1, Default::default());
    db.set_block_root_hash(LAST_BLOCK, Default::default());

    let mut restorer = RestoredTree::new(db.into());
    restorer.restore().await;
}
