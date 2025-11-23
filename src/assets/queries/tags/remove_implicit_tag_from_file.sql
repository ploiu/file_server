delete from TaggedItems
where fileId = ?1
  and tagId = ?2
  and implicitFromId is not null
