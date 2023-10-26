with query as (select fl.id, fl.name, fl.parentId
               from folders fl
               where parentId is null
               union all
               select f.id, query.name || '/' || f.name, f.parentId
               from folders f
                        join query on f.parentId = query.id)
select id, query.name as "path", parentId
from query
where id = ?1
