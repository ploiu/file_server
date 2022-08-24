select *
from folders;

select * from Folder_Files;

insert into folders(name, parentId)
values ('top', NULL),
       ('middle1', 1),
       ('middle2', 1),
       ('middle1_child1', 2),
       ('middle1_child2', 2);

insert into FileRecords(name, hash)
values ('marine_nanoless.jpg', 'blah'),
       ('98451688-large.gif', 'blah2'),
       ('anubis_bastet_houtengeki.jpg', 'blah3'),
       ('karin_rocket_cum.mp4', 'blah4'),
       ('ruler_akairiot.jpg', 'blah5');

insert into folder_files(folderId, fileId)
values (5, 1),
       (4, 3),
       (4, 5),
       (3, 2),
       (1, 4);

-- get folder for a file
select f.name, FR.name
from Folders f
         join Folder_Files FF on f.id = FF.folderId
         join FileRecords FR on FF.fileId = FR.id;

-- get full folder path
with query as (select fl.id, fl.name, fl.parentId
               from folders fl
               where parentId is null
               union all
               select f.id, query.name || '/' || f.name, f.parentId
               from folders f
                        join query
                             on f.parentId = query.id)
select query.name as "path"
from query where id = 5;

select * from FileRecords;