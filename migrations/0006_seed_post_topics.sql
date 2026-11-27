-- Enrich only untouched starter tag lists; custom editorial tags stay unchanged.
UPDATE posts
SET tags = '["Flutter","Architecture","Dart","State Management","Widget Testing"]',
    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
WHERE slug = 'state-that-does-not-fight-you'
  AND tags = '["Flutter","Architecture"]';

UPDATE posts
SET tags = '["Machine Learning","Flutter","TFLite","TensorFlow","On-Device AI","Image Classification"]',
    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
WHERE slug = 'plant-disease-on-device'
  AND tags = '["Machine Learning","Flutter","TFLite"]';

UPDATE posts
SET tags = '["Rust","Security","Web Security","Axum","Authentication"]',
    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
WHERE slug = 'the-browser-should-not-hold-the-keys'
  AND tags = '["Rust","Security"]';

UPDATE posts
SET tags = '["Mobile","Architecture","Flutter","Offline First","Data Synchronisation"]',
    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
WHERE slug IN ('offline-first-is-a-design-decision', 'offlineision')
  AND tags = '["Mobile","Architecture","Flutter"]';

UPDATE posts
SET tags = '["Design","Performance","Mobile","Web Performance","Progressive Enhancement"]',
    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
WHERE slug = 'designing-for-low-bandwidth'
  AND tags = '["Design","Performance","Mobile"]';

UPDATE posts
SET tags = '["Flutter","Release","DevOps","Android","App Store Deployment","Crash Reporting"]',
    updated_at = strftime('%Y-%m-%dT%H:%M:%SZ', 'now')
WHERE slug = 'a-checklist-for-shipping-flutter'
  AND tags = '["Flutter","Release","DevOps"]';
