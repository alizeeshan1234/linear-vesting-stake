import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { LinearStaking } from "../target/types/linear_staking";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  TOKEN_PROGRAM_ID,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";

describe("linear-staking", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.LinearStaking as Program<LinearStaking>;
  const admin = provider.wallet;

  // Test accounts
  let tokenMint: PublicKey;
  let adminTokenAccount: any;
  let stakeVault: PublicKey;
  let vaultTokenAccount: PublicKey;
  let transferAuthority: PublicKey;
  let userStake: PublicKey;
  let eventAuthority: PublicKey;

  // Use admin (provider.wallet) as the user - already funded
  const user = admin;

  // Constants (matching Rust seeds)
  const STAKE_VAULT_SEED = Buffer.from("stake_vault");
  const STAKE_VAULT_TOKEN_ACCOUNT_SEED = Buffer.from("stake_vault_token_account");
  const USER_STAKE_SEED = Buffer.from("user_stake");
  const TRANSFER_AUTHORITY_SEED = Buffer.from("transfer_authority");
  const EVENT_AUTHORITY_SEED = Buffer.from("__event_authority");

  // Test amounts
  const INITIAL_MINT_AMOUNT = 1_000_000_000_000; // 1000 tokens (9 decimals)
  const STAKE_AMOUNT = 100_000_000_000; // 100 tokens
  const UNSTAKE_AMOUNT = 50_000_000_000; // 50 tokens
  const REWARD_AMOUNT = 10_000_000_000; // 10 tokens

  // Short vesting period for testing (10 seconds)
  const TEST_VESTING_PERIOD = 10;

  // Helper function
  const sleep = (ms: number): Promise<void> => {
    return new Promise((resolve) => setTimeout(resolve, ms));
  };

  before(async () => {
    // Create token mint
    tokenMint = await createMint(
      provider.connection,
      (admin as any).payer,
      admin.publicKey,
      null,
      9
    );

    // Create admin/user token account and mint tokens (same wallet)
    adminTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      (admin as any).payer,
      tokenMint,
      admin.publicKey
    );

    await mintTo(
      provider.connection,
      (admin as any).payer,
      tokenMint,
      adminTokenAccount.address,
      admin.publicKey,
      INITIAL_MINT_AMOUNT * 2 // Double for both admin and user operations
    );

    // Derive PDAs
    [stakeVault] = PublicKey.findProgramAddressSync(
      [STAKE_VAULT_SEED],
      program.programId
    );

    [vaultTokenAccount] = PublicKey.findProgramAddressSync(
      [STAKE_VAULT_TOKEN_ACCOUNT_SEED],
      program.programId
    );

    [transferAuthority] = PublicKey.findProgramAddressSync(
      [TRANSFER_AUTHORITY_SEED],
      program.programId
    );

    [userStake] = PublicKey.findProgramAddressSync(
      [USER_STAKE_SEED, admin.publicKey.toBuffer()],
      program.programId
    );

    [eventAuthority] = PublicKey.findProgramAddressSync(
      [EVENT_AUTHORITY_SEED],
      program.programId
    );
  });

  it("1. should initialize the stake vault", async () => {
    const tx = await program.methods
      .initialize({
        vestingPeriod: new BN(TEST_VESTING_PERIOD),
      })
      .accountsStrict({
        admin: admin.publicKey,
        tokenMint: tokenMint,
        stakeVault: stakeVault,
        vaultTokenAccount: vaultTokenAccount,
        transferAuthority: transferAuthority,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    console.log("Initialize tx:", tx);

    const vaultState = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(vaultState.isInitialized, true);
    assert.equal(vaultState.admin.toString(), admin.publicKey.toString());
    assert.equal(vaultState.tokenMint.toString(), tokenMint.toString());
    assert.equal(vaultState.vestingPeriodSeconds.toNumber(), TEST_VESTING_PERIOD);
    assert.equal(vaultState.permissions.allowDeposits, true);
    assert.equal(vaultState.permissions.allowWithdrawals, true);
  });

  it("2. should deposit tokens into stake vault", async () => {
    const tx = await program.methods
      .depositStake({
        amount: new BN(STAKE_AMOUNT),
      })
      .accountsStrict({
        owner: user.publicKey,
        userTokenAccount: adminTokenAccount.address,
        stakeVault: stakeVault,
        vaultTokenAccount: vaultTokenAccount,
        userStake: userStake,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    console.log("Deposit stake tx:", tx);

    const userStakeState = await program.account.userStake.fetch(userStake);
    assert.equal(userStakeState.owner.toString(), user.publicKey.toString());
    assert.equal(userStakeState.stakedAmount.toNumber(), STAKE_AMOUNT);
    assert.equal(userStakeState.activeStakeAmount.toNumber(), STAKE_AMOUNT);
    assert.equal(userStakeState.isInitialized, true);

    const vaultState = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(vaultState.stakeStats.totalStaked.toNumber(), STAKE_AMOUNT);
    assert.equal(vaultState.stakeStats.activeAmount.toNumber(), STAKE_AMOUNT);
  });

  it("3. should allow additional deposits", async () => {
    const additionalAmount = 50_000_000_000;

    await program.methods
      .depositStake({
        amount: new BN(additionalAmount),
      })
      .accountsStrict({
        owner: user.publicKey,
        userTokenAccount: adminTokenAccount.address,
        stakeVault: stakeVault,
        vaultTokenAccount: vaultTokenAccount,
        userStake: userStake,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    const userStakeState = await program.account.userStake.fetch(userStake);
    assert.equal(
      userStakeState.stakedAmount.toNumber(),
      STAKE_AMOUNT + additionalAmount
    );
  });

  it("4. should fail deposit with zero amount", async () => {
    try {
      await program.methods
        .depositStake({
          amount: new BN(0),
        })
        .accountsStrict({
          owner: user.publicKey,
          userTokenAccount: adminTokenAccount.address,
          stakeVault: stakeVault,
          vaultTokenAccount: vaultTokenAccount,
          userStake: userStake,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          eventAuthority: eventAuthority,
          program: program.programId,
        })
        .rpc();

      assert.fail("Should have thrown an error");
    } catch (error: any) {
      assert.include(error.message, "InvalidAmount");
    }
  });

  it("5. should create an unstake request", async () => {
    const tx = await program.methods
      .unstakeRequest({
        amount: new BN(UNSTAKE_AMOUNT),
      })
      .accountsStrict({
        owner: user.publicKey,
        stakeVault: stakeVault,
        userStake: userStake,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    console.log("Unstake request tx:", tx);

    const userStakeState = await program.account.userStake.fetch(userStake);
    assert.equal(userStakeState.unstakeRequests.length, 1);
    assert.equal(
      userStakeState.unstakeRequests[0].totalAmount.toNumber(),
      UNSTAKE_AMOUNT
    );
    assert.equal(userStakeState.unstakeRequests[0].claimedAmount.toNumber(), 0);

    const vaultState = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(vaultState.stakeStats.unstakingAmount.toNumber(), UNSTAKE_AMOUNT);
  });

  it("6. should fail unstake with amount exceeding active stake", async () => {
    try {
      await program.methods
        .unstakeRequest({
          amount: new BN(1_000_000_000_000_000),
        })
        .accountsStrict({
          owner: user.publicKey,
          stakeVault: stakeVault,
          userStake: userStake,
          eventAuthority: eventAuthority,
          program: program.programId,
        })
        .rpc();

      assert.fail("Should have thrown an error");
    } catch (error: any) {
      assert.include(error.message, "InvalidAmount");
    }
  });

  it("7. should claim partially vested tokens", async () => {
    // Wait for partial vesting (5 seconds = 50%)
    console.log("Waiting 5 seconds for partial vesting...");
    await sleep(5000);

    const userBalanceBefore = await getAccount(
      provider.connection,
      adminTokenAccount.address
    );

    const tx = await program.methods
      .claimVested()
      .accountsStrict({
        owner: user.publicKey,
        userStake: userStake,
        stakeVault: stakeVault,
        userTokenAccount: adminTokenAccount.address,
        vaultTokenAccount: vaultTokenAccount,
        transferAuthority: transferAuthority,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    console.log("Claim vested tx:", tx);

    const userBalanceAfter = await getAccount(
      provider.connection,
      adminTokenAccount.address
    );

    const claimed = Number(userBalanceAfter.amount) - Number(userBalanceBefore.amount);
    console.log("Claimed amount:", claimed);
    assert.isAbove(claimed, 0, "Should have claimed some tokens");
  });

  it("8. should claim remaining vested tokens after full period", async () => {
    // Wait for remaining vesting period
    console.log("Waiting 6 seconds for full vesting...");
    await sleep(6000);

    const tx = await program.methods
      .claimVested()
      .accountsStrict({
        owner: user.publicKey,
        userStake: userStake,
        stakeVault: stakeVault,
        userTokenAccount: adminTokenAccount.address,
        vaultTokenAccount: vaultTokenAccount,
        transferAuthority: transferAuthority,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    console.log("Claim remaining vested tx:", tx);

    const userStakeStateAfter = await program.account.userStake.fetch(userStake);
    assert.equal(
      userStakeStateAfter.unstakeRequests.length,
      0,
      "Fully claimed request should be removed"
    );
  });

  it("9. should create and cancel an unstake request", async () => {
    // Create a new unstake request
    await program.methods
      .unstakeRequest({
        amount: new BN(UNSTAKE_AMOUNT),
      })
      .accountsStrict({
        owner: user.publicKey,
        stakeVault: stakeVault,
        userStake: userStake,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    const userStakeStateBefore = await program.account.userStake.fetch(userStake);
    const activeStakeBefore = userStakeStateBefore.activeStakeAmount.toNumber();

    // Cancel the unstake request
    const tx = await program.methods
      .cancelUnstake({
        requestIndex: 0,
      })
      .accountsStrict({
        owner: user.publicKey,
        stakeVault: stakeVault,
        userStake: userStake,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    console.log("Cancel unstake tx:", tx);

    const userStakeStateAfter = await program.account.userStake.fetch(userStake);

    assert.equal(
      userStakeStateAfter.activeStakeAmount.toNumber(),
      activeStakeBefore + UNSTAKE_AMOUNT
    );
    assert.equal(userStakeStateAfter.unstakeRequests.length, 0);
  });

  it("10. should fail cancel with invalid request index", async () => {
    try {
      await program.methods
        .cancelUnstake({
          requestIndex: 99,
        })
        .accountsStrict({
          owner: user.publicKey,
          stakeVault: stakeVault,
          userStake: userStake,
          eventAuthority: eventAuthority,
          program: program.programId,
        })
        .rpc();

      assert.fail("Should have thrown an error");
    } catch (error: any) {
      assert.include(error.message, "InvalidRequestIndex");
    }
  });

  it("11. should deposit rewards", async () => {
    const tx = await program.methods
      .depositRewards({
        amount: new BN(REWARD_AMOUNT),
      })
      .accountsStrict({
        admin: admin.publicKey,
        adminTokenAccount: adminTokenAccount.address,
        stakeVault: stakeVault,
        vaultTokenAccount: vaultTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    console.log("Deposit rewards tx:", tx);

    const vaultState = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(
      vaultState.rewardState.pendingRewards.toNumber(),
      REWARD_AMOUNT
    );
  });

  it("12. should distribute rewards", async () => {
    const tx = await program.methods
      .distributeRewards()
      .accountsStrict({
        payer: admin.publicKey,
        stakeVault: stakeVault,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    console.log("Distribute rewards tx:", tx);

    const vaultState = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(vaultState.rewardState.pendingRewards.toNumber(), 0);
    assert.isAbove(
      Number(vaultState.rewardState.rewardPerTokenStaked),
      0,
      "reward_per_token_staked should be updated"
    );
    assert.equal(
      Number(vaultState.rewardState.totalDistributed),
      REWARD_AMOUNT
    );
  });

  it("13. should collect rewards", async () => {
    const userBalanceBefore = await getAccount(
      provider.connection,
      adminTokenAccount.address
    );

    const tx = await program.methods
      .collectRewards()
      .accountsStrict({
        owner: user.publicKey,
        userStake: userStake,
        stakeVault: stakeVault,
        userTokenAccount: adminTokenAccount.address,
        vaultTokenAccount: vaultTokenAccount,
        transferAuthority: transferAuthority,
        tokenProgram: TOKEN_PROGRAM_ID,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    console.log("Collect rewards tx:", tx);

    const userBalanceAfter = await getAccount(
      provider.connection,
      adminTokenAccount.address
    );

    const rewardsClaimed =
      Number(userBalanceAfter.amount) - Number(userBalanceBefore.amount);
    console.log("Rewards claimed:", rewardsClaimed);
    assert.isAbove(rewardsClaimed, 0, "Should have claimed rewards");

    const userStakeState = await program.account.userStake.fetch(userStake);
    assert.equal(
      userStakeState.rewardState.unclaimedRewards.toNumber(),
      0,
      "Unclaimed rewards should be zero"
    );
  });

  it("14. should fail to collect when no rewards available", async () => {
    try {
      await program.methods
        .collectRewards()
        .accountsStrict({
          owner: user.publicKey,
          userStake: userStake,
          stakeVault: stakeVault,
          userTokenAccount: adminTokenAccount.address,
          vaultTokenAccount: vaultTokenAccount,
          transferAuthority: transferAuthority,
          tokenProgram: TOKEN_PROGRAM_ID,
          eventAuthority: eventAuthority,
          program: program.programId,
        })
        .rpc();

      assert.fail("Should have thrown an error");
    } catch (error: any) {
      assert.include(error.message, "NoRewardsToClaim");
    }
  });

  it("15. should handle multiple unstake requests", async () => {
    const amount1 = 10_000_000_000;
    const amount2 = 20_000_000_000;

    await program.methods
      .unstakeRequest({ amount: new BN(amount1) })
      .accountsStrict({
        owner: user.publicKey,
        stakeVault: stakeVault,
        userStake: userStake,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    await program.methods
      .unstakeRequest({ amount: new BN(amount2) })
      .accountsStrict({
        owner: user.publicKey,
        stakeVault: stakeVault,
        userStake: userStake,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    const userStakeState = await program.account.userStake.fetch(userStake);
    assert.equal(userStakeState.unstakeRequests.length, 2);
    console.log("Created 2 unstake requests successfully");
  });
});
