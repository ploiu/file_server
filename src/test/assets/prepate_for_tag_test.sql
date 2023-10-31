-- setup tables for tags stuff
delete
from Files_Tags;
delete
from Folders_Tags;
delete
from Folder_Files;
delete
from FileRecords;
delete
from Folders;
delete
from Tags;
delete
from sqlite_sequence;

insert into FileRecords(name)
values ('with_tags.txt'),                  -- 1
       ('without_tags.txt'),               -- 2
       ('in_folder_with_tags.txt'),        -- 3
       ('in_nested_folder_with_tags.txt'), -- 4
       ('in_folder_without_tags.txt'),     -- 5
       ('recursive_test.txt'); -- 6

insert into Folders(name, parentId)
values ('with tags', null),     -- 1
       ('without tags', null),  -- 2
       ('nested with tags', 1), -- 3
       ('deep nested', 3),      -- 4
       ('deeper nested', 4); -- 5

insert into Folder_Files(folderId, fileId)
values (2, 3), -- root/without tags/in_folder_with_tags
       (3, 4), -- root/with tags/nested with tags/in_nested_folder_with_tags
       (2, 5), -- root/without tags/in_folder_without_tags
       (5, 6); --  root/with tags/nested with tags/deep nested/deeper nested

insert into Tags(title)
values ('Tag1'), -- 1
       ('Tag2'), -- 2
       ('Tag3'), -- 3
       ('Tag4'), -- 4
       ('Tag5'); -- 5

insert into Files_Tags(fileRecordId, tagId)
values
    -- with tags
    (1, 1),
    (1, 2),
    -- in folder with tags
    (3, 3),
    (3, 1);

insert into Folders_Tags(folderId, tagId)
values
    -- with tags
    (1, 5),
    -- nested with tags
    (3, 4),
    (3, 3);

-- file 4 should return folders 3 and 1
