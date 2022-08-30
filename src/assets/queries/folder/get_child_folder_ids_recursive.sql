with query as (select f1.id
               from folders f1
               where f1.parentId = ?1
               union all
               select f2.id
               from folders f2
                        join query on f2.parentId = query.id)
select id
from query