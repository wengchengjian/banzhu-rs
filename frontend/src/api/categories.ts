import { client } from './client'

export interface CategoriesResponse {
  categories: string[]
}

export const categoriesApi = {
  list: () => client.get<CategoriesResponse>('/api/categories'),
}
