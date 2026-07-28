-- Seed data for contacts fixture database.
-- Entity types: 19=ABCDGroup, 22=ABCDContact, 25=CNCDContainer

INSERT INTO Z_PRIMARYKEY (Z_ENT, Z_NAME, Z_SUPER, Z_MAX) VALUES
  (1, 'ABCDAddressingGrammar', 0, 0),
  (11, 'ABCDEmailAddress', 0, 0),
  (12, 'ABCDLikeness', 0, 0),
  (14, 'ABCDNote', 0, 0),
  (15, 'ABCDPhoneNumber', 0, 0),
  (16, 'ABCDPostalAddress', 0, 0),
  (17, 'ABCDRecord', 0, 5),
  (19, 'ABCDGroup', 17, 1),
  (22, 'ABCDContact', 17, 1),
  (25, 'CNCDContainer', 17, 1),
  (16001, 'CHANGE', 0, 0),
  (16002, 'TRANSACTION', 0, 0),
  (16003, 'TRANSACTIONSTRING', 0, 0);

-- Container (On My Mac / iCloud local store)
INSERT INTO ZABCDRECORD (
  Z_PK, Z_ENT, Z_OPT, ZTYPE, ZUNIQUEID, ZNAME
) VALUES (
  1, 25, 1, 0,
  '11111111-1111-1111-1111-111111111111:ABContainer',
  'On My Mac'
);

-- Group
INSERT INTO ZABCDRECORD (
  Z_PK, Z_ENT, Z_OPT, ZCONTAINER, ZNAME, ZUNIQUEID
) VALUES (
  2, 19, 1, 1, 'Contacts',
  '22222222-2222-2222-2222-222222222222:ABGroup'
);

-- Contact: Jane Doe
INSERT INTO ZABCDRECORD (
  Z_PK, Z_ENT, Z_OPT, ZCONTAINER, ZFIRSTNAME, ZLASTNAME,
  ZSORTINGFIRSTNAME, ZSORTINGLASTNAME, ZUNIQUEID,
  ZCREATIONDATE, ZMODIFICATIONDATE
) VALUES (
  3, 22, 1, 1, 'Jane', 'Doe',
  'Jane', 'Doe',
  'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa:ABContact',
  1700000000, 1700000000
);

-- Group membership
INSERT INTO Z_22PARENTGROUPS (Z_22CONTACTS, Z_19PARENTGROUPS1) VALUES (3, 2);

-- Phone
INSERT INTO ZABCDPHONENUMBER (
  Z_PK, Z_ENT, Z_OPT, ZOWNER, ZFULLNUMBER, ZLABEL, ZUNIQUEID, ZORDERINGINDEX
) VALUES (
  1, 15, 1, 3, '+15551234567', '_$!<Mobile>!$_',
  'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb:ABPhoneNumber', 0
);

-- Email
INSERT INTO ZABCDEMAILADDRESS (
  Z_PK, Z_ENT, Z_OPT, ZOWNER, ZADDRESS, ZLABEL, ZUNIQUEID, ZORDERINGINDEX
) VALUES (
  1, 11, 1, 3, 'jane.doe@example.com', '_$!<Work>!$_',
  'cccccccc-cccc-cccc-cccc-cccccccccccc:ABEmailAddress', 0
);

-- Note
INSERT INTO ZABCDNOTE (Z_PK, Z_ENT, Z_OPT, ZCONTACT, ZTEXT) VALUES (
  1, 14, 1, 3, 'Fixture contact note'
);

-- Contact index for search
INSERT INTO ZABCDCONTACTINDEX (Z_PK, Z_ENT, Z_OPT, ZCONTACT, ZSTRINGFORINDEXING)
VALUES (1, 5, 1, 3, 'Jane Doe');
