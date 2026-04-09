-- Add optional tx_type column to zk_accounts
-- Stores 'ORDERTX' or 'LENDTX' when account is in Memo state, NULL when in Coin state
ALTER TABLE zk_accounts ADD COLUMN tx_type TEXT DEFAULT NULL;
