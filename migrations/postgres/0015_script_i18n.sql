-- Add i18n columns for script metadata localization
-- Stored as JSON: {"ja": "Japanese name", "de": "Deutscher Name"}
ALTER TABLE scripts ADD COLUMN name_i18n TEXT;
ALTER TABLE scripts ADD COLUMN description_i18n TEXT;
