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

describe("admin-instructions", () => {
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

  // Constants (matching Rust seeds)
  const STAKE_VAULT_SEED = Buffer.from("stake_vault");
  const STAKE_VAULT_TOKEN_ACCOUNT_SEED = Buffer.from("stake_vault_token_account");
  const USER_STAKE_SEED = Buffer.from("user_stake");
  const TRANSFER_AUTHORITY_SEED = Buffer.from("transfer_authority");
  const EVENT_AUTHORITY_SEED = Buffer.from("__event_authority");

  // Test amounts
  const INITIAL_MINT_AMOUNT = 1_000_000_000_000; // 1000 tokens (9 decimals)
  const STAKE_AMOUNT = 100_000_000_000; // 100 tokens

  // Initial vesting period for testing (30 seconds)
  const INITIAL_VESTING_PERIOD = 30;

  before(async () => {
    // Create token mint
    tokenMint = await createMint(
      provider.connection,
      (admin as any).payer,
      admin.publicKey,
      null,
      9
    );

    // Create admin token account and mint tokens
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
      INITIAL_MINT_AMOUNT
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
        vestingPeriod: new BN(INITIAL_VESTING_PERIOD),
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
    assert.equal(vaultState.isPaused, false);
    assert.equal(vaultState.vestingPeriodSeconds.toNumber(), INITIAL_VESTING_PERIOD);
  });

  it("2. should deposit tokens for testing", async () => {
    await program.methods
      .depositStake({
        amount: new BN(STAKE_AMOUNT),
      })
      .accountsStrict({
        owner: admin.publicKey,
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

    const vaultState = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(vaultState.stakeStats.totalStaked.toNumber(), STAKE_AMOUNT);
  });

  // =========================================================================
  // Pause/Unpause Tests
  // =========================================================================

  it("3. should pause the vault", async () => {
    const tx = await program.methods
      .pauseVault()
      .accountsStrict({
        admin: admin.publicKey,
        stakeVault: stakeVault,
      })
      .rpc();

    console.log("Pause vault tx:", tx);

    const vaultState = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(vaultState.isPaused, true);
  });

  it("4. should fail to pause when already paused", async () => {
    try {
      await program.methods
        .pauseVault()
        .accountsStrict({
          admin: admin.publicKey,
          stakeVault: stakeVault,
        })
        .rpc();

      assert.fail("Should have thrown an error");
    } catch (error: any) {
      assert.include(error.message, "VaultAlreadyPaused");
    }
  });

  it("5. should unpause the vault", async () => {
    const tx = await program.methods
      .unpauseVault()
      .accountsStrict({
        admin: admin.publicKey,
        stakeVault: stakeVault,
      })
      .rpc();

    console.log("Unpause vault tx:", tx);

    const vaultState = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(vaultState.isPaused, false);
  });

  it("6. should fail to unpause when not paused", async () => {
    try {
      await program.methods
        .unpauseVault()
        .accountsStrict({
          admin: admin.publicKey,
          stakeVault: stakeVault,
        })
        .rpc();

      assert.fail("Should have thrown an error");
    } catch (error: any) {
      assert.include(error.message, "NotPaused");
    }
  });

  // =========================================================================
  // Update Vesting Period Tests
  // =========================================================================

  it("7. should update vesting period", async () => {
    const NEW_VESTING_PERIOD = 60; // 60 seconds

    const tx = await program.methods
      .updateVestingPeriod({
        newVestingPeriodSeconds: new BN(NEW_VESTING_PERIOD),
      })
      .accountsStrict({
        admin: admin.publicKey,
        stakeVault: stakeVault,
      })
      .rpc();

    console.log("Update vesting period tx:", tx);

    const vaultState = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(vaultState.vestingPeriodSeconds.toNumber(), NEW_VESTING_PERIOD);
  });

  it("8. should fail to update vesting period to zero", async () => {
    try {
      await program.methods
        .updateVestingPeriod({
          newVestingPeriodSeconds: new BN(0),
        })
        .accountsStrict({
          admin: admin.publicKey,
          stakeVault: stakeVault,
        })
        .rpc();

      assert.fail("Should have thrown an error");
    } catch (error: any) {
      assert.include(error.message, "InvalidVestingPeriod");
    }
  });

  // =========================================================================
  // Emergency Withdraw Tests
  // =========================================================================

  it("9. should fail emergency withdraw when not paused", async () => {
    try {
      await program.methods
        .emergencyWithdraw({
          amount: new BN(STAKE_AMOUNT),
        })
        .accountsStrict({
          admin: admin.publicKey,
          stakeVault: stakeVault,
          vaultTokenAccount: vaultTokenAccount,
          adminTokenAccount: adminTokenAccount.address,
          transferAuthority: transferAuthority,
          tokenProgram: TOKEN_PROGRAM_ID,
          eventAuthority: eventAuthority,
          program: program.programId,
        })
        .rpc();

      assert.fail("Should have thrown an error");
    } catch (error: any) {
      assert.include(error.message, "VaultNotPaused");
    }
  });

  it("10. should emergency withdraw when paused", async () => {
    // First pause the vault
    await program.methods
      .pauseVault()
      .accountsStrict({
        admin: admin.publicKey,
        stakeVault: stakeVault,
      })
      .rpc();

    const vaultBalanceBefore = await getAccount(
      provider.connection,
      vaultTokenAccount
    );
    const adminBalanceBefore = await getAccount(
      provider.connection,
      adminTokenAccount.address
    );

    const withdrawAmount = STAKE_AMOUNT / 2; // Withdraw half

    const tx = await program.methods
      .emergencyWithdraw({
        amount: new BN(withdrawAmount),
      })
      .accountsStrict({
        admin: admin.publicKey,
        stakeVault: stakeVault,
        vaultTokenAccount: vaultTokenAccount,
        adminTokenAccount: adminTokenAccount.address,
        transferAuthority: transferAuthority,
        tokenProgram: TOKEN_PROGRAM_ID,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    console.log("Emergency withdraw tx:", tx);

    const vaultBalanceAfter = await getAccount(
      provider.connection,
      vaultTokenAccount
    );
    const adminBalanceAfter = await getAccount(
      provider.connection,
      adminTokenAccount.address
    );

    assert.equal(
      Number(vaultBalanceBefore.amount) - Number(vaultBalanceAfter.amount),
      withdrawAmount
    );
    assert.equal(
      Number(adminBalanceAfter.amount) - Number(adminBalanceBefore.amount),
      withdrawAmount
    );
  });

  it("11. should emergency withdraw all with amount=0", async () => {
    const vaultBalanceBefore = await getAccount(
      provider.connection,
      vaultTokenAccount
    );

    const tx = await program.methods
      .emergencyWithdraw({
        amount: new BN(0), // 0 means withdraw all
      })
      .accountsStrict({
        admin: admin.publicKey,
        stakeVault: stakeVault,
        vaultTokenAccount: vaultTokenAccount,
        adminTokenAccount: adminTokenAccount.address,
        transferAuthority: transferAuthority,
        tokenProgram: TOKEN_PROGRAM_ID,
        eventAuthority: eventAuthority,
        program: program.programId,
      })
      .rpc();

    console.log("Emergency withdraw all tx:", tx);

    const vaultBalanceAfter = await getAccount(
      provider.connection,
      vaultTokenAccount
    );

    assert.equal(Number(vaultBalanceAfter.amount), 0);
    console.log(
      "Withdrew all remaining tokens:",
      Number(vaultBalanceBefore.amount)
    );
  });

  it("12. should fail emergency withdraw with insufficient balance", async () => {
    try {
      await program.methods
        .emergencyWithdraw({
          amount: new BN(1_000_000_000_000), // More than vault has
        })
        .accountsStrict({
          admin: admin.publicKey,
          stakeVault: stakeVault,
          vaultTokenAccount: vaultTokenAccount,
          adminTokenAccount: adminTokenAccount.address,
          transferAuthority: transferAuthority,
          tokenProgram: TOKEN_PROGRAM_ID,
          eventAuthority: eventAuthority,
          program: program.programId,
        })
        .rpc();

      assert.fail("Should have thrown an error");
    } catch (error: any) {
      assert.include(error.message, "InsufficientVaultBalance");
    }
  });

  // =========================================================================
  // Update Permissions Tests
  // =========================================================================

  it("13. should update permissions - disable deposits", async () => {
    // First unpause the vault for permission tests
    await program.methods
      .unpauseVault()
      .accountsStrict({
        admin: admin.publicKey,
        stakeVault: stakeVault,
      })
      .rpc();

    const tx = await program.methods
      .updatePermissions({
        allowDeposits: false,
        allowWithdrawals: null,
      })
      .accountsStrict({
        admin: admin.publicKey,
        stakeVault: stakeVault,
      })
      .rpc();

    console.log("Update permissions (disable deposits) tx:", tx);

    const vaultState = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(vaultState.permissions.allowDeposits, false);
    assert.equal(vaultState.permissions.allowWithdrawals, true); // Should remain unchanged
  });

  it("14. should fail deposit when deposits are disabled", async () => {
    // Deposit some tokens to vault first for testing
    await mintTo(
      provider.connection,
      (admin as any).payer,
      tokenMint,
      adminTokenAccount.address,
      admin.publicKey,
      STAKE_AMOUNT
    );

    try {
      await program.methods
        .depositStake({
          amount: new BN(STAKE_AMOUNT),
        })
        .accountsStrict({
          owner: admin.publicKey,
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
      assert.include(error.message, "DepositsNotAllowed");
    }
  });

  it("15. should update permissions - disable withdrawals", async () => {
    const tx = await program.methods
      .updatePermissions({
        allowDeposits: null,
        allowWithdrawals: false,
      })
      .accountsStrict({
        admin: admin.publicKey,
        stakeVault: stakeVault,
      })
      .rpc();

    console.log("Update permissions (disable withdrawals) tx:", tx);

    const vaultState = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(vaultState.permissions.allowDeposits, false); // Should remain unchanged
    assert.equal(vaultState.permissions.allowWithdrawals, false);
  });

  it("16. should update permissions - enable both", async () => {
    const tx = await program.methods
      .updatePermissions({
        allowDeposits: true,
        allowWithdrawals: true,
      })
      .accountsStrict({
        admin: admin.publicKey,
        stakeVault: stakeVault,
      })
      .rpc();

    console.log("Update permissions (enable both) tx:", tx);

    const vaultState = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(vaultState.permissions.allowDeposits, true);
    assert.equal(vaultState.permissions.allowWithdrawals, true);
  });

  it("17. should deposit after re-enabling deposits", async () => {
    const vaultStateBefore = await program.account.stakeVault.fetch(stakeVault);
    const totalStakedBefore = vaultStateBefore.stakeStats.totalStaked.toNumber();

    const tx = await program.methods
      .depositStake({
        amount: new BN(STAKE_AMOUNT),
      })
      .accountsStrict({
        owner: admin.publicKey,
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

    console.log("Deposit after re-enabling tx:", tx);

    const vaultStateAfter = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(
      vaultStateAfter.stakeStats.totalStaked.toNumber(),
      totalStakedBefore + STAKE_AMOUNT
    );
  });

  it("18. should update permissions with no changes (both null)", async () => {
    const vaultStateBefore = await program.account.stakeVault.fetch(stakeVault);

    const tx = await program.methods
      .updatePermissions({
        allowDeposits: null,
        allowWithdrawals: null,
      })
      .accountsStrict({
        admin: admin.publicKey,
        stakeVault: stakeVault,
      })
      .rpc();

    console.log("Update permissions (no changes) tx:", tx);

    const vaultStateAfter = await program.account.stakeVault.fetch(stakeVault);
    assert.equal(
      vaultStateAfter.permissions.allowDeposits,
      vaultStateBefore.permissions.allowDeposits
    );
    assert.equal(
      vaultStateAfter.permissions.allowWithdrawals,
      vaultStateBefore.permissions.allowWithdrawals
    );
  });
});
