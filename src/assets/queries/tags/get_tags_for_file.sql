select
    t.*,
    ti.impliedFromId
from
    Tags t
    join TaggedItems ti on t.id = ti.tagId
where
    ti.fileId = ?1