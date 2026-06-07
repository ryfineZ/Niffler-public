CREATE INDEX IF NOT EXISTS idx_entitlement_usage_request
    ON public.entitlement_usage_ledgers USING btree (request_id);
