with query as (select fl.id, fl.name, fl.parentId
               from folders fl
               where parentId is null
               union all
               select f.id, query.name || '/' || f.name, f.parentId
               from folders f
                        join query on f.parentId = query.id)
select query.id, query.name as "path", query.parentId
from query
where query.parentId = ?1