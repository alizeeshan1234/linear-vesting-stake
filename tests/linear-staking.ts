import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { LinearStaking } from "../target/types/linear_staking";
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  TOKEN_PROGRAM_ID,
  mintTo,
  getAccount
} from "@solana/spl-token";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import { assert } from "chai";

// Helper function to sleep
const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

describe("linear-staking", () => {
  let provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.linearStaking as Program<LinearStaking>;

  let tokenMint: PublicKey;
  let stakeVaultPDA: PublicKey;
  let vaultTokenAccountPDA: PublicKey;
  let transferAuthorityPDA: PublicKey;
  let userTokenAccountPDA: PublicKey;
  let userStakeAccountPDA: PublicKey;

  // Use a shorter vesting period for testing (10 seconds)
  const VESTING_PERIOD_SECONDS = 10;
  const INITIAL_MINT_AMOUNT = 10000;
  const STAKE_AMOUNT = 1000;

  before(async () => {
    // Create a new mint for testing
    tokenMint = await createMint(
      provider.connection,
      provider.wallet.payer,
      provider.wallet.publicKey,
      null,
      9, // 9 decimals
    );

    let userTokenAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      provider.wallet.payer,
      tokenMint,
      provider.wallet.publicKey
    );

    userTokenAccountPDA = userTokenAccount.address;

    // Derive PDAs
    [stakeVaultPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("stake_vault")],
      program.programId
    );

    [vaultTokenAccountPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("stake_vault_token_account")],
      program.programId
    );

    [transferAuthorityPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("transfer_authority")],
      program.programId
    );

    [userStakeAccountPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from("user_stake"), provider.wallet.publicKey.toBuffer()],
      program.programId
    );

    // Mint tokens to user
    await mintTo(
      provider.connection,
      provider.wallet.payer,
      tokenMint,
      userTokenAccountPDA,
      provider.wallet.publicKey,
      INITIAL_MINT_AMOUNT
    );
  });

  describe("Initialization", () => {
    it("Initialize stake vault", async () => {
      let initializeParams = {
        vestingPeriod: new anchor.BN(VESTING_PERIOD_SECONDS)
      };

      const tx = await program.methods.initialize(initializeParams).accountsPartial({
        admin: provider.wallet.publicKey,
        tokenMint: tokenMint,
        stakeVault: stakeVaultPDA,
        vaultTokenAccount: vaultTokenAccountPDA,
        transferAuthority: transferAuthorityPDA,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID
      }).signers([provider.wallet.payer]).rpc();

      console.log("Initialization tx:", tx);

      // Verify vault state
      const vaultState = await program.account.stakeVault.fetch(stakeVaultPDA);
      assert.isTrue(vaultState.isInitialized);
      assert.equal(vaultState.vestingPeriod.toNumber(), VESTING_PERIOD_SECONDS);
      assert.equal(vaultState.stakeStats.totalStaked.toNumber(), 0);
      assert.equal(vaultState.stakeStats.activeAmount.toNumber(), 0);
      assert.equal(vaultState.stakeStats.unstakingAmount.toNumber(), 0);
      assert.equal(vaultState.stakeStats.totalVested.toNumber(), 0);
    });
  });

  describe("Staking", () => {
    it("Deposit stake", async () => {
      let depositStakeParams = {
        amount: new anchor.BN(STAKE_AMOUNT)
      };

      const tx = await program.methods.depositStake(depositStakeParams).accountsPartial({
        owner: provider.wallet.publicKey,
        feePayer: provider.wallet.publicKey,
        userTokenAccount: userTokenAccountPDA,
        stakeVault: stakeVaultPDA,
        vaultTokenAccount: vaultTokenAccountPDA,
        userStake: userStakeAccountPDA,
        tokenMint: tokenMint,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([provider.wallet.payer]).rpc();

      console.log("Deposit stake tx:", tx);

      // Verify user stake state
      const userStakeState = await program.account.userStake.fetch(userStakeAccountPDA);
      assert.equal(userStakeState.activeStakeAmount.toNumber(), STAKE_AMOUNT);
      assert.isTrue(userStakeState.isInitialized);

      // Verify vault state
      const vaultState = await program.account.stakeVault.fetch(stakeVaultPDA);
      assert.equal(vaultState.stakeStats.totalStaked.toNumber(), STAKE_AMOUNT);
      assert.equal(vaultState.stakeStats.activeAmount.toNumber(), STAKE_AMOUNT);
    });

    it("Deposit additional stake", async () => {
      const additionalAmount = 500;
      let depositStakeParams = {
        amount: new anchor.BN(additionalAmount)
      };

      await program.methods.depositStake(depositStakeParams).accountsPartial({
        owner: provider.wallet.publicKey,
        feePayer: provider.wallet.publicKey,
        userTokenAccount: userTokenAccountPDA,
        stakeVault: stakeVaultPDA,
        vaultTokenAccount: vaultTokenAccountPDA,
        userStake: userStakeAccountPDA,
        tokenMint: tokenMint,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([provider.wallet.payer]).rpc();

      // Verify cumulative stake
      const userStakeState = await program.account.userStake.fetch(userStakeAccountPDA);
      assert.equal(userStakeState.activeStakeAmount.toNumber(), STAKE_AMOUNT + additionalAmount);

      const vaultState = await program.account.stakeVault.fetch(stakeVaultPDA);
      assert.equal(vaultState.stakeStats.totalStaked.toNumber(), STAKE_AMOUNT + additionalAmount);
    });
  });

  describe("Reward Distribution", () => {
    const REWARD_AMOUNT = 100;

    it("Admin deposits rewards", async () => {
      let depositRewardsParams = {
        amount: new anchor.BN(REWARD_AMOUNT)
      };

      const tx = await program.methods.depositRewards(depositRewardsParams).accountsPartial({
        admin: provider.wallet.publicKey,
        adminTokenAccount: userTokenAccountPDA,
        stakeVault: stakeVaultPDA,
        vaultTokenAccount: vaultTokenAccountPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([provider.wallet.payer]).rpc();

      console.log("Deposit rewards tx:", tx);

      // Verify pending rewards
      const vaultState = await program.account.stakeVault.fetch(stakeVaultPDA);
      assert.equal(vaultState.rewardState.pendingRewards.toNumber(), REWARD_AMOUNT);
    });

    it("Distribute rewards to accumulator", async () => {
      let distributeRewardsParams = {};

      const tx = await program.methods.distributeRewards(distributeRewardsParams).accountsPartial({
        payer: provider.wallet.publicKey,
        stakeVault: stakeVaultPDA,
      }).signers([provider.wallet.payer]).rpc();

      console.log("Distribute rewards tx:", tx);

      // Verify rewards distributed
      const vaultState = await program.account.stakeVault.fetch(stakeVaultPDA);
      assert.equal(vaultState.rewardState.pendingRewards.toNumber(), 0);
      assert.isAbove(vaultState.rewardState.rewardPerTokenStaked.toNumber(), 0);
      assert.equal(vaultState.rewardState.totalDistributed.toNumber(), REWARD_AMOUNT);
    });

    it("User collects rewards", async () => {
      let collectRewardsParams = {};

      const userTokenBefore = await getAccount(provider.connection, userTokenAccountPDA);

      const tx = await program.methods.collectRewards(collectRewardsParams).accountsPartial({
        owner: provider.wallet.publicKey,
        userTokenAccount: userTokenAccountPDA,
        stakeVault: stakeVaultPDA,
        vaultTokenAccount: vaultTokenAccountPDA,
        userStake: userStakeAccountPDA,
        transferAuthority: transferAuthorityPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([provider.wallet.payer]).rpc();

      console.log("Collect rewards tx:", tx);

      // Verify rewards received
      const userTokenAfter = await getAccount(provider.connection, userTokenAccountPDA);
      const rewardsReceived = Number(userTokenAfter.amount) - Number(userTokenBefore.amount);

      console.log(`Rewards received: ${rewardsReceived}`);
      assert.isAbove(rewardsReceived, 0);

      // Verify user reward state
      const userStakeState = await program.account.userStake.fetch(userStakeAccountPDA);
      assert.equal(userStakeState.rewardState.unclaimedRewards.toNumber(), 0);
      assert.isAbove(userStakeState.rewardState.totalClaimed.toNumber(), 0);
    });
  });

  describe("Unstaking with Linear Vesting", () => {
    const UNSTAKE_AMOUNT = 500;

    it("Request unstake", async () => {
      let unstakeRequestParams = {
        amount: new anchor.BN(UNSTAKE_AMOUNT)
      };

      const vaultStateBefore = await program.account.stakeVault.fetch(stakeVaultPDA);
      const activeAmountBefore = vaultStateBefore.stakeStats.activeAmount.toNumber();

      const tx = await program.methods.unstakeRequest(unstakeRequestParams).accountsPartial({
        owner: provider.wallet.publicKey,
        stakeVault: stakeVaultPDA,
        userStake: userStakeAccountPDA,
      }).signers([provider.wallet.payer]).rpc();

      console.log("Unstake request tx:", tx);

      // Verify user stake state
      const userStakeState = await program.account.userStake.fetch(userStakeAccountPDA);
      assert.equal(userStakeState.unstakeRequestCount, 1);
      assert.equal(userStakeState.unstakeRequests[0].totalAmount.toNumber(), UNSTAKE_AMOUNT);
      assert.equal(userStakeState.unstakeRequests[0].claimedAmount.toNumber(), 0);

      // Verify vault state
      const vaultState = await program.account.stakeVault.fetch(stakeVaultPDA);
      assert.equal(vaultState.stakeStats.activeAmount.toNumber(), activeAmountBefore - UNSTAKE_AMOUNT);
      assert.equal(vaultState.stakeStats.unstakingAmount.toNumber(), UNSTAKE_AMOUNT);
      // total_staked should remain unchanged (tokens still in vault)
      assert.equal(vaultState.stakeStats.totalStaked.toNumber(), vaultStateBefore.stakeStats.totalStaked.toNumber());
    });

    it("Claim partial vested tokens (50%)", async () => {
      // Wait for half the vesting period
      const halfPeriod = Math.floor(VESTING_PERIOD_SECONDS / 2);
      console.log(`Waiting ${halfPeriod} seconds (50% of vesting period)...`);
      await sleep(halfPeriod * 1000);

      let claimVestedParams = {
        requestId: null
      };

      const userTokenBefore = await getAccount(provider.connection, userTokenAccountPDA);
      const vaultStateBefore = await program.account.stakeVault.fetch(stakeVaultPDA);

      const tx = await program.methods.claimVested(claimVestedParams).accountsPartial({
        owner: provider.wallet.publicKey,
        userTokenAccount: userTokenAccountPDA,
        stakeVault: stakeVaultPDA,
        vaultTokenAccount: vaultTokenAccountPDA,
        transferAuthority: transferAuthorityPDA,
        userStake: userStakeAccountPDA,
        tokenMint: tokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([provider.wallet.payer]).rpc();

      console.log("Partial claim tx:", tx);

      // Verify tokens received (~50%)
      const userTokenAfter = await getAccount(provider.connection, userTokenAccountPDA);
      const tokensReceived = Number(userTokenAfter.amount) - Number(userTokenBefore.amount);
      console.log(`Tokens claimed (partial): ${tokensReceived}`);

      // Should be approximately 50% (with some tolerance for timing)
      assert.isAbove(tokensReceived, UNSTAKE_AMOUNT * 0.3);
      assert.isAtMost(tokensReceived, UNSTAKE_AMOUNT * 0.8);

      // Verify vault stats updated
      const vaultState = await program.account.stakeVault.fetch(stakeVaultPDA);
      assert.isAbove(vaultState.stakeStats.totalVested.toNumber(), 0);
      assert.isBelow(vaultState.stakeStats.totalStaked.toNumber(), vaultStateBefore.stakeStats.totalStaked.toNumber());
    });

    it("Claim remaining vested tokens", async () => {
      // Wait for the rest of the vesting period
      const remainingTime = Math.ceil(VESTING_PERIOD_SECONDS / 2) + 1;
      console.log(`Waiting ${remainingTime} seconds for remaining vesting...`);
      await sleep(remainingTime * 1000);

      let claimVestedParams = {
        requestId: null
      };

      const userStakeBefore = await program.account.userStake.fetch(userStakeAccountPDA);
      const remainingToClaim = userStakeBefore.unstakeRequests[0].totalAmount.toNumber() -
                               userStakeBefore.unstakeRequests[0].claimedAmount.toNumber();

      const tx = await program.methods.claimVested(claimVestedParams).accountsPartial({
        owner: provider.wallet.publicKey,
        userTokenAccount: userTokenAccountPDA,
        stakeVault: stakeVaultPDA,
        vaultTokenAccount: vaultTokenAccountPDA,
        transferAuthority: transferAuthorityPDA,
        userStake: userStakeAccountPDA,
        tokenMint: tokenMint,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([provider.wallet.payer]).rpc();

      console.log("Final claim tx:", tx);

      // Verify unstake request cleaned up
      const userStakeState = await program.account.userStake.fetch(userStakeAccountPDA);
      assert.equal(userStakeState.unstakeRequestCount, 0);

      // Verify total vested
      const vaultState = await program.account.stakeVault.fetch(stakeVaultPDA);
      assert.equal(vaultState.stakeStats.totalVested.toNumber(), UNSTAKE_AMOUNT);
      assert.equal(vaultState.stakeStats.unstakingAmount.toNumber(), 0);
    });
  });

  describe("Cancel Unstake", () => {
    const UNSTAKE_AMOUNT = 300;

    it("Create unstake request to cancel", async () => {
      let unstakeRequestParams = {
        amount: new anchor.BN(UNSTAKE_AMOUNT)
      };

      await program.methods.unstakeRequest(unstakeRequestParams).accountsPartial({
        owner: provider.wallet.publicKey,
        stakeVault: stakeVaultPDA,
        userStake: userStakeAccountPDA,
      }).signers([provider.wallet.payer]).rpc();

      const userStakeState = await program.account.userStake.fetch(userStakeAccountPDA);
      assert.equal(userStakeState.unstakeRequestCount, 1);
    });

    it("Cancel unstake request", async () => {
      let cancelUnstakeParams = {
        requestId: 0
      };

      const vaultStateBefore = await program.account.stakeVault.fetch(stakeVaultPDA);
      const userStakeBefore = await program.account.userStake.fetch(userStakeAccountPDA);

      const tx = await program.methods.cancelUnstake(cancelUnstakeParams).accountsPartial({
        owner: provider.wallet.publicKey,
        stakeVault: stakeVaultPDA,
        userStake: userStakeAccountPDA,
      }).signers([provider.wallet.payer]).rpc();

      console.log("Cancel unstake tx:", tx);

      // Verify request cancelled
      const userStakeState = await program.account.userStake.fetch(userStakeAccountPDA);
      assert.equal(userStakeState.unstakeRequestCount, 0);

      // Verify tokens returned to active stake
      assert.equal(
        userStakeState.activeStakeAmount.toNumber(),
        userStakeBefore.activeStakeAmount.toNumber() + UNSTAKE_AMOUNT
      );

      // Verify vault stats
      const vaultState = await program.account.stakeVault.fetch(stakeVaultPDA);
      assert.equal(
        vaultState.stakeStats.activeAmount.toNumber(),
        vaultStateBefore.stakeStats.activeAmount.toNumber() + UNSTAKE_AMOUNT
      );
      assert.equal(vaultState.stakeStats.unstakingAmount.toNumber(), 0);
    });
  });
});