select
    ti.id,
    ti.fileId,
    ti.folderId,
    ti.implicitFromId,
    t.id,
    t.title
from
    Tags t
    join TaggedItems ti on t.id = ti.tagId
where
    ti.folderId = ?1