import { getCurrentInstance } from 'vue';

export default function useVM() {
  return getCurrentInstance()?.proxy;
}
