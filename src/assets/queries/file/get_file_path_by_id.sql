with query as (select fl.id, fl.name, fl.parentId
               from folders fl
               where parentId is null
               union all
               select f.id, query.name || '/' || f.name, f.parentId
               from folders f
                        join query on f.parentId = query.id)
select coalesce(query.name || '/', '') || f.name
from FileRecords f
         left join Folder_Files FF on f.id = FF.fileId
         left join query on query.id = ff.folderId
where f.id = ?1