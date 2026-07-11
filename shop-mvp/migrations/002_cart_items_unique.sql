-- =============================================================================
-- 🔒 Adaugă UNIQUE constraint pe (session_id, product_slug, price_bani)
-- Prevint duplicatele în coș (race condition la add_item concurent)
-- =============================================================================

-- Mai întâi curățăm duplicatele existente (păstrăm doar cel mai recent rând)
DELETE FROM cart_items a USING (
    SELECT session_id, product_slug, price_bani, MAX(created_at) as max_created
    FROM cart_items
    GROUP BY session_id, product_slug, price_bani
    HAVING COUNT(*) > 1
) b
WHERE a.session_id = b.session_id
  AND a.product_slug = b.product_slug
  AND a.price_bani = b.price_bani
  AND a.created_at < b.max_created;

-- Apoi adăugăm UNIQUE constraint
ALTER TABLE cart_items
ADD CONSTRAINT cart_items_unique_session_product_price
UNIQUE (session_id, product_slug, price_bani);
