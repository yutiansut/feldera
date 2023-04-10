import React, { memo, useEffect, useState } from 'react'
import { NodeProps } from 'reactflow'
import useSqlPlaceholderClick from '../hooks/useSqlPlaceholderClick'
import { Icon } from '@iconify/react'
import { Autocomplete, CardContent, TextField, Typography } from '@mui/material'
import { PlaceholderNode } from '../NodeTypes'
import { useQuery } from '@tanstack/react-query'
import { ProjectService } from 'src/types/manager/services/ProjectService'
import { projectToProjectWithSchema, ProjectWithSchema } from 'src/types/program'

const SqlPlaceHolderNode = (props: NodeProps) => {
  const [programs, setPrograms] = useState<ProjectWithSchema[]>([])
  const placeHolderReplace = useSqlPlaceholderClick(props.id)
  const onProgramSelected = (e: any, v: string) => {
    const program = programs.find(p => p.name == v)
    if (program != undefined) {
      placeHolderReplace(e, program)
    }
  }

  const { isLoading, isError, data } = useQuery({ queryKey: ['project'], queryFn: ProjectService.listProjects })
  useEffect(() => {
    if (!isLoading && !isError) {
      setPrograms(data.filter(p => p.schema).map(projectToProjectWithSchema))
    }
  }, [isLoading, isError, data])

  return (
    <PlaceholderNode>
      <CardContent sx={{ textAlign: 'center', '& svg': { mb: 2 } }}>
        <Icon icon={props.data.icon} fontSize='2rem' />
        <Typography variant='h6' sx={{ mb: 4 }}>
          {props.data.label}
        </Typography>

        <Autocomplete
          z-index={20}
          onInputChange={onProgramSelected}
          disableCloseOnSelect
          options={programs.map(p => p.name)}
          getOptionLabel={option => option}
          renderInput={params => (
            <TextField {...params} className='nodrag' label='Program' placeholder='Search for SQL...' />
          )}
        />
      </CardContent>
    </PlaceholderNode>
  )
}

export default memo(SqlPlaceHolderNode)