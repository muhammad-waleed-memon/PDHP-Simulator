import { describe, it, expect } from 'vitest'
import { normalizeRole, formatRoleDisplay, ROLE_MAP } from './client'

describe('Role normalization and display contract', () => {
  it('normalizes uppercase and PascalCase roles to backend enum variants', () => {
    expect(normalizeRole('PATIENT')).toBe('Patient')
    expect(normalizeRole('DOCTOR')).toBe('Doctor')
    expect(normalizeRole('LAB')).toBe('Lab')
    expect(normalizeRole('FACILITY_ADMIN')).toBe('FacilityAdmin')
    expect(normalizeRole('SYSTEM_ADMIN')).toBe('SystemAdmin')

    expect(normalizeRole('Patient')).toBe('Patient')
    expect(normalizeRole('Doctor')).toBe('Doctor')
    expect(normalizeRole('Lab')).toBe('Lab')
    expect(normalizeRole('FacilityAdmin')).toBe('FacilityAdmin')
    expect(normalizeRole('SystemAdmin')).toBe('SystemAdmin')
  })

  it('formats roles for UI display cleanly', () => {
    expect(formatRoleDisplay('Patient')).toBe('PATIENT')
    expect(formatRoleDisplay('Doctor')).toBe('DOCTOR')
    expect(formatRoleDisplay('Lab')).toBe('LAB')
    expect(formatRoleDisplay('FacilityAdmin')).toBe('FACILITY_ADMIN')
    expect(formatRoleDisplay('SystemAdmin')).toBe('SYSTEM_ADMIN')

    expect(formatRoleDisplay('PATIENT')).toBe('PATIENT')
    expect(formatRoleDisplay('DOCTOR')).toBe('DOCTOR')
  })

  it('contains complete mapping dictionary for all 5 system roles', () => {
    expect(ROLE_MAP['PATIENT']).toBe('Patient')
    expect(ROLE_MAP['DOCTOR']).toBe('Doctor')
    expect(ROLE_MAP['LAB']).toBe('Lab')
    expect(ROLE_MAP['FACILITY_ADMIN']).toBe('FacilityAdmin')
    expect(ROLE_MAP['SYSTEM_ADMIN']).toBe('SystemAdmin')
  })
})
