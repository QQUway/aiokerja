-- Datasheet extraction + comparison (AIDC hardware: RFID readers/antennas,
-- handheld computers, barcode/RFID printers, scanners). `attributes` jsonb on
-- products already holds the extracted specs; this adds the structured
-- identity fields used for the catalog and comparison views.

ALTER TABLE products
    ADD COLUMN brand text,
    ADD COLUMN model text,
    ADD COLUMN device_type text,
    ADD COLUMN source_document_id uuid REFERENCES documents(id) ON DELETE SET NULL;

CREATE INDEX products_device_type_idx ON products (device_type);
CREATE INDEX products_source_document_idx ON products (source_document_id);
