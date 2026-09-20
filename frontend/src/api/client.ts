/**
 * API client — base Axios instance configured for the backend.
 * All API modules should import from this file.
 */

import axios from 'axios'
import { tokenStorage } from './tokenStorage'

const API_BASE_URL = (import.meta as any).env?.VITE_API_BASE_URL ?? '/api/v1'

export const apiClient = axios.create({
  baseURL: API_BASE_URL,
  headers: {
    'Content-Type': 'application/json',
    'Accept': 'application/json',
  },
  timeout: 15000,
})

// Request interceptor — attach JWT from tokenStorage abstraction if present
apiClient.interceptors.request.use((config) => {
  const token = tokenStorage.getToken()
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

// Response interceptor — handle 401 globally
apiClient.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      tokenStorage.clearToken()
    }
    return Promise.reject(error)
  }
)

// ── Types ─────────────────────────────────────────────────────────

export type UserRole = 'PATIENT' | 'DOCTOR' | 'LAB' | 'FACILITY_ADMIN' | 'SYSTEM_ADMIN'

export interface UserMe {
  id: string
  email: string
  full_name: string
  role: UserRole
  status: string
  created_at: string
}

export interface AuthResponse {
  token: string
  token_type: string
  expires_in: number
  user: {
    id: string
    email: string
    full_name: string
    role: UserRole
    status: string
  }
}

export interface PatientRecord {
  id: string
  digital_health_id: string
  user_id?: string
  full_name: string
  date_of_birth: string
  gender: string
  blood_group?: string
  phone_number?: string
  address?: string
  city?: string
  province?: string
  emergency_contact_name?: string
  emergency_contact_phone?: string
  status: string
  created_at: string
}

export interface CreatePatientRequest {
  email?: string
  password?: string
  full_name: string
  date_of_birth: string
  gender: string
  blood_group?: string
  phone_number?: string
  address?: string
  city?: string
  province?: string
  emergency_contact_name?: string
  emergency_contact_phone?: string
}

// ── Health API ────────────────────────────────────────────────────

export interface HealthResponse {
  status: string
  version: string
  timestamp: string
}

export interface ReadinessResponse extends HealthResponse {
  checks: {
    database: {
      status: string
      message?: string
    }
  }
}

export const healthApi = {
  liveness: () =>
    apiClient.get<HealthResponse>('/health', { baseURL: '' }).then((r) => r.data),
  readiness: () =>
    apiClient.get<ReadinessResponse>('/health/ready', { baseURL: '' }).then((r) => r.data),
}

// ── Auth API ──────────────────────────────────────────────────────

export const authApi = {
  login: (credentials: { email: string; password: string }) =>
    apiClient.post<AuthResponse>('/auth/login', credentials).then((r) => r.data),
  
  register: (payload: { email: string; password: string; full_name: string; role: UserRole }) =>
    apiClient.post<AuthResponse>('/auth/register', payload).then((r) => r.data),
  
  getMe: () =>
    apiClient.get<UserMe>('/auth/me').then((r) => r.data),
}

// ── Patients API ──────────────────────────────────────────────────

export const patientsApi = {
  registerPatient: (payload: CreatePatientRequest) =>
    apiClient.post<PatientRecord>('/patients', payload).then((r) => r.data),

  getById: (id: string) =>
    apiClient.get<PatientRecord>(`/patients/${id}`).then((r) => r.data),

  getByDigitalHealthId: (dhid: string) =>
    apiClient.get<PatientRecord>(`/patients/by-digital-health-id/${encodeURIComponent(dhid)}`).then((r) => r.data),
}

// ── Phase 4 Clinical Types & APIs ──────────────────────────────────

export interface Medication {
  id: string
  name: string
  generic_name?: string
  strength?: string
  dosage_form?: string
  route?: string
  status: string
}

export interface Condition {
  id: string
  patient_id: string
  name: string
  code?: string
  status: string
  onset_date?: string
  resolved_date?: string
  notes?: string
  version: number
  created_at: string
}

export interface Allergy {
  id: string
  patient_id: string
  allergen: string
  reaction?: string
  severity: string
  status: string
  recorded_date: string
  notes?: string
  version: number
  created_at: string
}

export interface Encounter {
  id: string
  patient_id: string
  doctor_id: string
  facility_id: string
  encounter_type: string
  start_time: string
  end_time?: string
  reason: string
  clinical_notes?: string
  status: string
}

export interface PrescriptionItemDetail {
  item: {
    id: string
    prescription_id: string
    medication_id: string
    dosage: string
    frequency: string
    route?: string
    duration: string
    quantity: number
    instructions?: string
  }
  medication_name: string
}

export interface PrescriptionWithDetails {
  prescription: {
    id: string
    patient_id: string
    doctor_id: string
    issued_at: string
    status: string
    clinical_notes?: string
    override_reason?: string
  }
  items: PrescriptionItemDetail[]
}

export interface LabReport {
  id: string
  patient_id: string
  test_name: string
  result: string
  unit?: string
  reference_range?: string
  abnormal_flag: boolean
  report_date: string
  status: string
}

export interface TimelineItem {
  id: string
  event_type: string
  title: string
  description: string
  status: string
  timestamp: string
}

export const clinicalApi = {
  getMedications: () => apiClient.get<Medication[]>('/medications').then((r) => r.data),
  getConditions: (patientId: string) =>
    apiClient.get<Condition[]>(`/patients/${patientId}/conditions`).then((r) => r.data),
  createCondition: (patientId: string, data: any) =>
    apiClient.post<Condition>(`/patients/${patientId}/conditions`, data).then((r) => r.data),
  getAllergies: (patientId: string) =>
    apiClient.get<Allergy[]>(`/patients/${patientId}/allergies`).then((r) => r.data),
  createAllergy: (patientId: string, data: any) =>
    apiClient.post<Allergy>(`/patients/${patientId}/allergies`, data).then((r) => r.data),
  getEncounters: (patientId: string) =>
    apiClient.get<Encounter[]>(`/patients/${patientId}/encounters`).then((r) => r.data),
  createEncounter: (patientId: string, data: any) =>
    apiClient.post<Encounter>(`/patients/${patientId}/encounters`, data).then((r) => r.data),
  getPrescriptions: (patientId: string) =>
    apiClient.get<PrescriptionWithDetails[]>(`/patients/${patientId}/prescriptions`).then((r) => r.data),
  createPrescription: (patientId: string, data: any) =>
    apiClient.post<PrescriptionWithDetails>(`/patients/${patientId}/prescriptions`, data).then((r) => r.data),
  getLabReports: (patientId: string) =>
    apiClient.get<LabReport[]>(`/patients/${patientId}/lab-reports`).then((r) => r.data),
  createLabReport: (patientId: string, data: any) =>
    apiClient.post<LabReport>(`/patients/${patientId}/lab-reports`, data).then((r) => r.data),
  getTimeline: (patientId: string) =>
    apiClient.get<TimelineItem[]>(`/patients/${patientId}/timeline`).then((r) => r.data),
}

// ── Phase 5 FHIR Interoperability API ──────────────────────────────

export const fhirApi = {
  getPatient: (patientId: string) =>
    apiClient.get(`/fhir/Patient/${patientId}`, { headers: { Accept: 'application/fhir+json' } }).then((r) => r.data),
  getPatientEverything: (patientId: string) =>
    apiClient.get(`/fhir/Patient/${patientId}/$everything`, { headers: { Accept: 'application/fhir+json' } }).then((r) => r.data),
}

