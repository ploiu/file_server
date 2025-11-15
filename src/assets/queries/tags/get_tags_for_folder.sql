select
    t.*,
    ti.inheritedFromId
from
    Tags t
    join TaggedItems ti on t.id = ti.tagId
where
    ti.folderId = ?1