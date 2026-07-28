pub const CONTAINER_SELECT: &str = "\
SELECT r.Z_PK AS row_id, r.ZUNIQUEID AS unique_id, r.ZNAME AS name, r.ZTYPE AS container_type \
FROM ZABCDRECORD r \
WHERE r.Z_ENT = 25";

pub const GROUP_SELECT: &str = "\
SELECT r.Z_PK AS row_id, r.ZUNIQUEID AS unique_id, r.ZNAME AS name, r.ZCONTAINER AS container_row_id, \
       c.ZUNIQUEID AS container_unique_id, r.ZTYPE AS group_type \
FROM ZABCDRECORD r \
LEFT JOIN ZABCDRECORD c ON c.Z_PK = r.ZCONTAINER \
WHERE r.Z_ENT = 19";

pub const CONTACT_SELECT: &str = "\
SELECT r.Z_PK AS row_id, r.ZUNIQUEID AS unique_id, r.ZFIRSTNAME AS first_name, r.ZLASTNAME AS last_name, \
       r.ZMIDDLENAME AS middle_name, r.ZNICKNAME AS nickname, r.ZORGANIZATION AS organization, \
       r.ZJOBTITLE AS job_title, r.ZDEPARTMENT AS department, r.ZNAME AS display_name, \
       r.ZCONTAINER AS container_row_id, c.ZUNIQUEID AS container_unique_id, \
       CAST(r.ZCREATIONDATE AS REAL) AS creation_date, \
       CAST(r.ZMODIFICATIONDATE AS REAL) AS modification_date, \
       CAST(r.ZBIRTHDAY AS REAL) AS birthday, n.ZTEXT AS note_text, \
       CASE WHEN l.ZDATA IS NOT NULL OR r.ZIMAGEDATA IS NOT NULL THEN 1 ELSE 0 END AS has_photo \
FROM ZABCDRECORD r \
LEFT JOIN ZABCDRECORD c ON c.Z_PK = r.ZCONTAINER \
LEFT JOIN ZABCDNOTE n ON n.ZCONTACT = r.Z_PK \
LEFT JOIN ZABCDLIKENESS l ON l.ZOWNER = r.Z_PK AND l.ZISPRIMARY = 1 \
WHERE r.Z_ENT = 22";

pub const PHONE_SELECT: &str = "\
SELECT ZUNIQUEID AS unique_id, ZFULLNUMBER AS number, ZLABEL AS label, ZISPRIMARY AS is_primary, ZORDERINGINDEX AS ordering_index \
FROM ZABCDPHONENUMBER WHERE ZOWNER = ? ORDER BY ZORDERINGINDEX";

pub const EMAIL_SELECT: &str = "\
SELECT ZUNIQUEID AS unique_id, ZADDRESS AS address, ZLABEL AS label, ZISPRIMARY AS is_primary, ZORDERINGINDEX AS ordering_index \
FROM ZABCDEMAILADDRESS WHERE ZOWNER = ? ORDER BY ZORDERINGINDEX";

pub const ADDRESS_SELECT: &str = "\
SELECT ZUNIQUEID AS unique_id, ZSTREET AS street, ZCITY AS city, ZSTATE AS state, ZZIPCODE AS postal_code, \
       ZCOUNTRYNAME AS country, ZLABEL AS label, ZISPRIMARY AS is_primary, ZORDERINGINDEX AS ordering_index \
FROM ZABCDPOSTALADDRESS WHERE ZOWNER = ? ORDER BY ZORDERINGINDEX";

pub const URL_SELECT: &str = "\
SELECT ZUNIQUEID AS unique_id, ZURL AS url, ZLABEL AS label, ZISPRIMARY AS is_primary, ZORDERINGINDEX AS ordering_index \
FROM ZABCDURLADDRESS WHERE ZOWNER = ? ORDER BY ZORDERINGINDEX";

pub const SOCIAL_SELECT: &str = "\
SELECT ZUNIQUEID AS unique_id, ZSERVICENAME AS service, ZUSERNAME AS username, ZURLSTRING AS url, \
       ZLABEL AS label, ZISPRIMARY AS is_primary, ZORDERINGINDEX AS ordering_index \
FROM ZABCDSOCIALPROFILE WHERE ZOWNER = ? ORDER BY ZORDERINGINDEX";

pub const GROUP_IDS_FOR_CONTACT: &str = "\
SELECT g.ZUNIQUEID AS unique_id \
FROM Z_22PARENTGROUPS pg \
JOIN ZABCDRECORD g ON g.Z_PK = pg.Z_19PARENTGROUPS1 \
WHERE pg.Z_22CONTACTS = ?";

pub const PHOTO_SELECT: &str = "\
SELECT COALESCE(l.ZDATA, r.ZIMAGEDATA) AS photo_data, r.ZIMAGETYPE AS image_type \
FROM ZABCDRECORD r \
LEFT JOIN ZABCDLIKENESS l ON l.ZOWNER = r.Z_PK AND l.ZISPRIMARY = 1 \
WHERE r.Z_ENT = 22 AND lower(r.ZUNIQUEID) LIKE lower(?) || '%' \
LIMIT 1";

pub const CONTAINER_RESOLVE_SELECT: &str = "\
SELECT lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)), r.ZNAME, r.ZTYPE \
FROM ZABCDRECORD r \
WHERE r.Z_ENT = 25 AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) = lower(?)";

pub const GROUP_RESOLVE_SELECT: &str = "\
SELECT lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)), r.ZNAME, r.ZTYPE, r.ZCONTAINER \
FROM ZABCDRECORD r \
WHERE r.Z_ENT = 19 AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) = lower(?)";

pub const CONTACT_EXTERNAL_ID_SELECT: &str = "\
SELECT substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1) \
FROM ZABCDRECORD r \
WHERE r.Z_ENT = 22 AND lower(substr(r.ZUNIQUEID, 1, instr(r.ZUNIQUEID, ':') - 1)) = lower(?)";
