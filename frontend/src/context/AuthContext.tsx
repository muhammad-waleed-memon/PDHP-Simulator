import React, { createContext, useContext, useEffect, useState } from 'react'
import { authApi, type UserMe } from '../api/client'
import { tokenStorage } from '../api/tokenStorage'

interface AuthContextType {
  user: UserMe | null
  token: string | null
  isLoading: boolean
  login: (email: string, password: string) => Promise<void>
  logout: () => void
  refreshUser: () => Promise<void>
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

export const AuthProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [user, setUser] = useState<UserMe | null>(null)
  const [token, setToken] = useState<string | null>(tokenStorage.getToken())
  const [isLoading, setIsLoading] = useState<boolean>(true)

  const fetchCurrentUser = async () => {
    const currentToken = tokenStorage.getToken()
    if (!currentToken) {
      setUser(null)
      setToken(null)
      setIsLoading(false)
      return
    }

    try {
      const userData = await authApi.getMe()
      setUser(userData)
      setToken(currentToken)
    } catch {
      tokenStorage.clearToken()
      setUser(null)
      setToken(null)
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    fetchCurrentUser()
  }, [])

  const login = async (email: string, password: string) => {
    setIsLoading(true)
    try {
      const res = await authApi.login({ email, password })
      tokenStorage.setToken(res.token)
      setToken(res.token)
      setUser({
        id: res.user.id,
        email: res.user.email,
        full_name: res.user.full_name,
        role: res.user.role,
        status: res.user.status,
        created_at: new Date().toISOString(),
      })
    } finally {
      setIsLoading(false)
    }
  }

  const logout = () => {
    tokenStorage.clearToken()
    setToken(null)
    setUser(null)
  }

  return (
    <AuthContext.Provider
      value={{
        user,
        token,
        isLoading,
        login,
        logout,
        refreshUser: fetchCurrentUser,
      }}
    >
      {children}
    </AuthContext.Provider>
  )
}

export const useAuth = () => {
  const context = useContext(AuthContext)
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}
